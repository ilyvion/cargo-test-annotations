// Copyright 2022 Alexander Krivács Schrøder
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// OR
//
// Licensed under the MIT License. See LICENSE-MIT for details.

use cargo_metadata::{Artifact, Message, MessageIter, Metadata, Package};
use miette::{Diagnostic, IntoDiagnostic};
use regex::Regex;
use std::io::{BufRead, Read};
use std::iter::Peekable;
use std::str::FromStr;
use thiserror::Error;

pub fn parse<R: Read>(r: R, metadata: Metadata) -> miette::Result<Vec<TestRun>> {
    let workspace_packages = metadata.workspace_packages();
    let reader = std::io::BufReader::new(r);

    let mut current_artifact = None;
    let mut test_runs = Vec::new();
    let mut message_iter = Message::parse_stream(reader).peekable();
    while let Some(message) = message_iter.next() {
        match message.into_diagnostic()? {
            Message::CompilerArtifact(
                artifact @ Artifact {
                    executable: Some(_),
                    ..
                },
            ) => {
                current_artifact = Some(artifact);
            }
            Message::TextLine(_) => {
                let artifact = current_artifact
                    .as_ref()
                    .expect("No current artifact. Is the input data malformed?");
                let package = workspace_packages
                    .iter()
                    .copied()
                    .find(|&w| w.id == artifact.package_id)
                    .ok_or_else(|| {
                        miette::miette!(
                            "could not find package '{}' from test in workspace",
                            artifact.package_id
                        )
                    })?;
                let features = artifact.features.clone();
                let test_run_parser = TestRunParser::new(package.clone(), features);
                test_runs.push(test_run_parser.parse(&mut message_iter)?);
                while let Some(Ok(Message::TextLine(_))) = message_iter.peek() {
                    let _ = message_iter.next();
                }
            }
            _ => {} // Irrelevant messages
        }
    }

    Ok(test_runs)
}

#[macro_export]
macro_rules! parse_capture {
    (let $var:ident: $type:ty = $cap:expr) => {
        parse_capture!(let $var: $type = $cap => stringify!($var))
    };
    (let $var:ident: $type:ty = $cap:expr => $name:expr) => {
        let $var: $type;
        parse_capture!($var => $cap => $name)
    };
    ($var:expr => $cap:expr) => {
        parse_capture!($var => $cap => stringify!($var))
    };
    ($var:expr => $cap:expr => $name:expr) => {
        $var = $cap
            .name($name)
            .expect(concat!("<", $name, ">"))
            .as_str()
            .parse()
            .into_diagnostic()?;
    };
}

#[derive(Clone, Debug)]
pub struct TestRun {
    pub package: Package,
    pub features: Vec<String>,
    pub test_run: TestData,
    pub doc_test_run: TestData,
}
impl From<TestRunParser> for TestRun {
    fn from(parser: TestRunParser) -> Self {
        let TestRunParser {
            package,
            features,
            test_run,
            doc_test_run,
            ..
        } = parser;
        Self {
            package,
            features,
            test_run: test_run.map(From::from).unwrap(),
            doc_test_run: doc_test_run.map(From::from).unwrap(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TestData {
    pub test_count: usize,
    pub test_results: Vec<TestResult>,
    pub test_summary: TestSummary,
}
impl From<TestDataParseResult> for TestData {
    fn from(r: TestDataParseResult) -> Self {
        let TestDataParseResult {
            test_count,
            test_results,
            test_summary,
        } = r;
        Self {
            test_count,
            test_results: test_results.into_iter().map(From::from).collect(),
            test_summary,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TestResult {
    pub name: String,
    pub result: TestResultValue,
}
impl From<TestResultParseResult> for TestResult {
    fn from(t: TestResultParseResult) -> Self {
        let TestResultParseResult {
            name,
            kind,
            failure_info,
        } = t;
        let result = match kind {
            TestResultKind::Ok => TestResultValue::Ok,
            TestResultKind::Failed => TestResultValue::Failed(failure_info.unwrap()),
        };
        Self { name, result }
    }
}

#[derive(Clone, Debug)]
pub enum TestResultValue {
    Ok,
    Failed(TestFailureInfo),
}

impl TestResultValue {
    pub fn unwrap_failure(self) -> TestFailureInfo {
        match self {
            Self::Failed(failure) => failure,
            Self::Ok => panic!("called `TestResultValue::unwrap_failure()` on an `Ok` value"),
        }
    }
    pub fn unwrap_failure_ref(&self) -> &TestFailureInfo {
        match self {
            Self::Failed(failure) => failure,
            Self::Ok => panic!("called `TestResultValue::unwrap_failure()` on an `Ok` value"),
        }
    }
}

#[derive(Debug)]
struct TestRunParser {
    package: Package,
    features: Vec<String>,
    phase: TestRunParserPhase,
    state: TestRunParserState,
    test_count: usize,
    test_results: Vec<TestResultParseResult>,
    test_run: Option<TestDataParseResult>,
    doc_test_run: Option<TestDataParseResult>,
}

impl TestRunParser {
    fn new(package: Package, features: Vec<String>) -> Self {
        Self {
            package,
            features,
            state: TestRunParserState::Initial,
            phase: TestRunParserPhase::Tests,
            test_count: 0,
            test_results: Vec::new(),
            test_run: None,
            doc_test_run: None,
        }
    }

    pub fn parse<R: BufRead>(
        mut self,
        message_iter: &mut Peekable<MessageIter<R>>,
    ) -> miette::Result<TestRun> {
        while self.phase != TestRunParserPhase::Done {
            while self.state != TestRunParserState::Done {
                let message = message_iter
                    .next()
                    .expect("we're in the middle of parsing")
                    .into_diagnostic()?;
                match message {
                    Message::TextLine(text) => match self.state {
                        TestRunParserState::Initial => {
                            RUNNING_REGEX.with(|r| -> miette::Result<()> {
                                if let Some(c) = r.captures(&text) {
                                    parse_capture!(let test_count: usize = c => "count");
                                    self.test_count = test_count;
                                    if test_count > 0 {
                                        self.state = TestRunParserState::Tests;
                                    } else {
                                        self.state = TestRunParserState::Results;
                                    }
                                }

                                Ok(())
                            })?;
                        }
                        TestRunParserState::Tests => {
                            TEST_REGEX.with(|r| -> miette::Result<()> {
                                if let Some(c) = r.captures(&text) {
                                    parse_capture!(let name: String = c);
                                    parse_capture!(let result: TestResultKind = c);
                                    self.test_results
                                        .push(TestResultParseResult::new(name, result));
                                } else {
                                    #[allow(clippy::collapsible_else_if)]
                                    if self
                                        .test_results
                                        .iter()
                                        .any(|r| r.kind == TestResultKind::Failed)
                                    {
                                        self.state = TestRunParserState::FailuresOutput;
                                    } else {
                                        self.state = TestRunParserState::Results;
                                    }
                                }

                                Ok(())
                            })?;
                        }
                        TestRunParserState::FailuresOutput => {
                            let mut more_failures = true;
                            let mut text = text;
                            let mut failure_parsing_state = TestRunFailureParserState::Initial;
                            while more_failures {
                                let mut text_inner = text;
                                let mut name = None;
                                let mut panic_text = None;
                                let mut location = None;
                                let mut stacktrace = None;
                                while failure_parsing_state != TestRunFailureParserState::Done {
                                    match failure_parsing_state {
                                        TestRunFailureParserState::Initial => {
                                            if text_inner.trim() == "failures:" {
                                                failure_parsing_state =
                                                    TestRunFailureParserState::Header;
                                            }
                                        }
                                        TestRunFailureParserState::Header => {
                                            FAILURE_HEADER_REGEX.with(
                                                |r| -> miette::Result<()> {
                                                    if let Some(c) = r.captures(&text_inner) {
                                                        parse_capture!(let n: String = c => "name");

                                                        name = Some(n);
                                                        failure_parsing_state =
                                                            TestRunFailureParserState::Panic;
                                                    }

                                                    Ok(())
                                                },
                                            )?;
                                        }
                                        TestRunFailureParserState::Panic => {
                                            while let Some(Ok(Message::TextLine(t))) =
                                                message_iter.peek()
                                            {
                                                if t.trim() != "stack backtrace:" {
                                                    text_inner.push('\n');
                                                    text_inner.push_str(t);
                                                    let _ = message_iter.next();
                                                } else {
                                                    let rpos = text_inner
                                                        .rfind(',')
                                                        .expect("regular panic format");
                                                    let (pt, l) = text_inner.split_at(rpos);
                                                    let l = l
                                                        .strip_prefix(", ")
                                                        .expect("regular panic format")
                                                        .trim();
                                                    panic_text = Some(pt.to_owned());
                                                    location = Some(l.to_owned());

                                                    failure_parsing_state =
                                                        TestRunFailureParserState::Stacktrace;
                                                    break;
                                                }
                                            }
                                        }
                                        TestRunFailureParserState::Stacktrace => {
                                            while let Some(Ok(Message::TextLine(t))) =
                                                message_iter.peek()
                                            {
                                                if !t.trim().is_empty() {
                                                    text_inner.push('\n');
                                                    text_inner.push_str(t);
                                                    let _ = message_iter.next();
                                                } else {
                                                    stacktrace = Some(text_inner.clone());

                                                    failure_parsing_state =
                                                        TestRunFailureParserState::Done;
                                                    break;
                                                }
                                            }
                                        }
                                        TestRunFailureParserState::Done => unreachable!(),
                                    }
                                    match message_iter
                                        .next()
                                        .expect("we're not done")
                                        .expect("we're not done")
                                    {
                                        Message::TextLine(t) => {
                                            text_inner = t;
                                        }
                                        m => {
                                            miette::bail!(
                                                "Encountered unexpected message: {:?} while parser was in state {:?}",
                                                m,
                                                self
                                            )
                                        }
                                    }
                                }
                                let failure_info = TestFailureInfo::new(
                                    panic_text.unwrap(),
                                    location.as_ref().unwrap().parse().into_diagnostic()?,
                                    stacktrace.unwrap(),
                                );
                                let result = self
                                    .test_results
                                    .iter_mut()
                                    .find(|r| r.name == name.as_deref().unwrap())
                                    .unwrap();
                                result.failure_info = Some(failure_info);

                                FAILURE_HEADER_REGEX.with(|r| {
                                    while let Some(Ok(Message::TextLine(t))) = message_iter.peek() {
                                        if r.is_match(t) {
                                            failure_parsing_state =
                                                TestRunFailureParserState::Header;
                                            break;
                                        } else if t.trim() == "failures:" {
                                            more_failures = false;
                                            break;
                                        } else {
                                            let _ = message_iter.next();
                                        }
                                    }
                                });
                                text = String::new();
                            }

                            self.state = TestRunParserState::FailuresListing;
                        }
                        TestRunParserState::FailuresListing => {
                            while let Some(Ok(Message::TextLine(t))) = message_iter.peek() {
                                if !t.trim().is_empty() {
                                    let _ = message_iter.next();
                                } else {
                                    self.state = TestRunParserState::Results;
                                    break;
                                }
                            }
                        }
                        TestRunParserState::Results => {
                            RESULT_REGEX.with(|r| -> miette::Result<()> {
                                if let Some(c) = r.captures(&text) {
                                    parse_capture!(let result: TestResultKind = c);
                                    parse_capture!(let passed: usize = c);
                                    parse_capture!(let failed: usize = c);
                                    parse_capture!(let ignored: usize = c);
                                    parse_capture!(let measured: usize = c);
                                    parse_capture!(let filtered: usize = c);
                                    parse_capture!(let time: String = c);

                                    let test_summary = TestSummary::new(
                                        result, passed, failed, ignored, measured, filtered, time,
                                    );
                                    let test_run = TestDataParseResult::new(
                                        self.test_count,
                                        std::mem::take(&mut self.test_results),
                                        test_summary,
                                    );
                                    match self.phase {
                                        TestRunParserPhase::Tests => {
                                            self.test_run = Some(test_run);
                                        }
                                        TestRunParserPhase::DocTests => {
                                            self.doc_test_run = Some(test_run);
                                        }
                                        TestRunParserPhase::Done => unreachable!(),
                                    }
                                    self.state = TestRunParserState::Done;
                                }

                                Ok(())
                            })?;
                        }
                        TestRunParserState::Done => unreachable!(),
                    },
                    m => miette::bail!(
                        "Encountered unexpected message: {:?} while parser was in state {:?}",
                        m,
                        self
                    ),
                }
            }
            match self.phase {
                TestRunParserPhase::Tests => {
                    self.phase = TestRunParserPhase::DocTests;
                    self.state = TestRunParserState::Initial;
                    self.test_count = 0;
                }
                TestRunParserPhase::DocTests => self.phase = TestRunParserPhase::Done,
                TestRunParserPhase::Done => unreachable!(),
            }
        }
        Ok(self.into())
    }
}

#[derive(Clone, Debug)]
struct TestDataParseResult {
    test_count: usize,
    test_results: Vec<TestResultParseResult>,
    test_summary: TestSummary,
}

impl TestDataParseResult {
    fn new(
        test_count: usize,
        test_results: Vec<TestResultParseResult>,
        test_summary: TestSummary,
    ) -> Self {
        Self {
            test_count,
            test_results,
            test_summary,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TestRunParserState {
    Initial,
    Tests,
    FailuresOutput,
    FailuresListing,
    Results,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TestRunFailureParserState {
    Initial,
    Header,
    Panic,
    Stacktrace,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TestRunParserPhase {
    Tests,
    DocTests,
    Done,
}

#[derive(Clone, Debug)]
struct TestResultParseResult {
    name: String,
    kind: TestResultKind,
    failure_info: Option<TestFailureInfo>,
}

impl TestResultParseResult {
    fn new(name: String, kind: TestResultKind) -> Self {
        Self {
            name,
            kind,
            failure_info: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TestResultKind {
    Ok,
    Failed,
}
impl FromStr for TestResultKind {
    type Err = TestResultKindParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ok" => Ok(Self::Ok),
            "FAILED" => Ok(Self::Failed),
            other => Err(TestResultKindParseError(other.into())),
        }
    }
}

#[derive(Error, Diagnostic, Debug)]
#[error("Unknown test result: {0}")]
pub struct TestResultKindParseError(String);

#[derive(Clone, Debug)]
pub struct TestFailureInfo {
    pub panic_text: String,
    pub location: TestFailureLocation,
    pub stacktrace: String,
}

impl TestFailureInfo {
    fn new(panic_text: String, location: TestFailureLocation, stacktrace: String) -> Self {
        Self {
            panic_text,
            location,
            stacktrace,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TestFailureLocation {
    pub file: String,
    pub line: u64,
    pub column: u64,
}

impl FromStr for TestFailureLocation {
    type Err = TestFailureLocationParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(':');
        let file = parts
            .next()
            .ok_or_else(|| TestFailureLocationParseError(s.to_owned()))?
            .to_owned();
        let line: u64 = parts
            .next()
            .ok_or_else(|| TestFailureLocationParseError(s.to_owned()))?
            .parse()
            .map_err(|_| TestFailureLocationParseError(s.to_owned()))?;
        let column: u64 = parts
            .next()
            .ok_or_else(|| TestFailureLocationParseError(s.to_owned()))?
            .parse()
            .map_err(|_| TestFailureLocationParseError(s.to_owned()))?;

        Ok(Self { file, line, column })
    }
}

#[derive(Error, Diagnostic, Debug)]
#[error("Unknown location format: {0}")]
pub struct TestFailureLocationParseError(String);

#[derive(Clone, Debug)]
pub struct TestSummary {
    pub result: TestResultKind,
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
    pub measured: usize,
    pub filtered: usize,
    pub time: String,
}

impl TestSummary {
    fn new(
        result: TestResultKind,
        passed: usize,
        failed: usize,
        ignored: usize,
        measured: usize,
        filtered: usize,
        time: String,
    ) -> Self {
        Self {
            result,
            passed,
            failed,
            ignored,
            measured,
            filtered,
            time,
        }
    }
}

thread_local! {
    static RUNNING_REGEX: Regex = Regex::new(r"running (?P<count>\d+) tests?").unwrap();
    static TEST_REGEX: Regex = Regex::new(r"test (?P<name>.+?) ... (?P<result>ok|FAILED)").unwrap();
    static RESULT_REGEX: Regex = Regex::new(r"test result: (?P<result>ok|FAILED). (?P<passed>\d+) passed; (?P<failed>\d+) failed; (?P<ignored>\d+) ignored; (?P<measured>\d+) measured; (?P<filtered>\d+) filtered out; finished in (?P<time>.+)").unwrap();
    static FAILURE_HEADER_REGEX: Regex = Regex::new(r"---- (?P<name>.+?) stdout ----").unwrap();
}
