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

use cargo_metadata::MetadataCommand;
use cargo_test_annotations::{parse_capture, TestResultValue};
use miette::{Context, IntoDiagnostic};
use regex::Regex;

fn main() -> miette::Result<()> {
    let mut args = std::env::args();
    args.next(); // skip executable
    let metadata_output_path = args
        .next()
        .expect("first argument to program is path to file of metadata output");
    let test_output_path = args
        .next()
        .expect("second argument to program is path to file of test output");

    let metadata = MetadataCommand::parse(
        std::fs::read_to_string(&metadata_output_path)
            .into_diagnostic()
            .with_context(|| metadata_output_path)?,
    )
    .into_diagnostic()?;

    let test_output_file = std::fs::File::open(&test_output_path)
        .into_diagnostic()
        .with_context(|| test_output_path)?;

    let test_runs = cargo_test_annotations::parse(test_output_file, metadata)?;
    for test_run in test_runs
        .into_iter()
        .filter(|r| r.test_run.test_count != 0 || r.doc_test_run.test_count != 0)
        .filter(|r| {
            r.test_run
                .test_results
                .iter()
                .any(|t| matches!(t.result, TestResultValue::Failed(_)))
                || r.doc_test_run
                    .test_results
                    .iter()
                    .any(|t| matches!(t.result, TestResultValue::Failed(_)))
        })
    {
        let features = test_run.features;

        for result in test_run
            .test_run
            .test_results
            .iter()
            .filter(|t| matches!(t.result, TestResultValue::Failed(_)))
        {
            let failure = result.result.unwrap_failure_ref();
            let location = &failure.location;
            println!(
                "::error file={},line={},col={},title={}::features: [{}]\\n\\n{}",
                location.file,
                location.line,
                location.column,
                failure
                    .panic_text
                    .replace("\r\n", "\n")
                    .replace('\r', "\n")
                    .replace('\n', "\\n"),
                features.join(", "),
                failure
                    .stacktrace
                    .replace("\r\n", "\n")
                    .replace('\r', "\n")
                    .replace('\n', "\\n"),
            );
        }
        for result in test_run
            .doc_test_run
            .test_results
            .iter()
            .filter(|t| matches!(t.result, TestResultValue::Failed(_)))
        {
            // TODO: Handle the (line N) part of doc tests to show the correct location of the test.

            let failure = result.result.unwrap_failure_ref();
            let location = &failure.location;

            let (_, real_line, real_column) =
                DOCTEST_NAME_FILE_REGEX.with(|r| -> miette::Result<(String, usize, usize)> {
                    if let Some(c) = r.captures(&result.name) {
                        parse_capture!(let file: String = c);
                        parse_capture!(let line: usize = c);

                        let real_line = location.line + line - 3;
                        let real_column = location.column + 4;
                        return Ok((file, real_line, real_column));
                    }
                    miette::bail!("Doctest title in unexpected format: {}", &result.name);
                })?;

            println!(
                "::error file={},line={},col={},title={}::features: [{}]\\n\\n{}\\n{}",
                location.file,
                real_line,
                real_column,
                result.name,
                features.join(", "),
                failure
                    .panic_text
                    .replace("\r\n", "\n")
                    .replace('\r', "\n")
                    .replace('\n', "\\n"),
                failure
                    .stacktrace
                    .replace("\r\n", "\n")
                    .replace('\r', "\n")
                    .replace('\n', "\\n"),
            );
        }
    }

    Ok(())
}

thread_local! {
    static DOCTEST_NAME_FILE_REGEX: Regex = Regex::new(r"(?P<file>.+?) - \(line (?P<line>\d+)\)").unwrap();
}
