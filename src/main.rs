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
use chrono::Utc;
use miette::{Context, IntoDiagnostic};
use octocrab::params::checks::{
    CheckRunConclusion, CheckRunOutput, CheckRunOutputAnnotation, CheckRunOutputAnnotationLevel,
    CheckRunStatus,
};
use octocrab::OctocrabBuilder;
use regex::Regex;

#[tokio::main]
async fn main() -> miette::Result<()> {
    let metadata = std::env::var("INPUT_METADATA").expect("`metadata` input value missing");
    let tests = std::env::var("INPUT_TESTS").expect("`tests` input value missing");
    let token = std::env::var("INPUT_TOKEN").expect("`token` input value missing");
    let name = std::env::var("INPUT_NAME").expect("`name` input value missing");

    let metadata = MetadataCommand::parse(
        std::fs::read_to_string(&metadata)
            .into_diagnostic()
            .with_context(|| metadata)?,
    )
    .into_diagnostic()?;

    let test_output_file = std::fs::File::open(&tests)
        .into_diagnostic()
        .with_context(|| tests)?;

    let octocrab = octocrab::initialise(
        OctocrabBuilder::new()
            .user_access_token(token)
            .build()
            .expect("valid configuration"),
    );

    let test_runs = cargo_test_annotations::parse(test_output_file, metadata)?;
    let mut annotations = Vec::new();
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
            .into_iter()
            .filter(|t| matches!(t.result, TestResultValue::Failed(_)))
        {
            let failure = result.result.unwrap_failure_ref();
            let location = &failure.location;

            annotations.push(CheckRunOutputAnnotation {
                annotation_level: CheckRunOutputAnnotationLevel::Failure,
                path: location.file.clone(),
                start_line: location.line as u32,
                end_line: location.line as u32,
                start_column: Some(location.column as u32),
                end_column: None,
                message: format!(
                    r#"features: [{}]

cause:
{}

{}"#,
                    features.join(", "),
                    failure.panic_text.replace("\r\n", "\n").replace('\r', "\n"),
                    failure.stacktrace.replace("\r\n", "\n").replace('\r', "\n")
                ),
                title: Some(result.name.clone()),
                raw_details: Some(format!("{:#?}", result)),
            })
        }
        for result in test_run
            .doc_test_run
            .test_results
            .iter()
            .filter(|t| matches!(t.result, TestResultValue::Failed(_)))
        {
            let failure = result.result.unwrap_failure_ref();
            let location = &failure.location;

            let (_, real_line, real_column) =
                DOCTEST_NAME_FILE_REGEX.with(|r| -> miette::Result<(String, u64, u64)> {
                    if let Some(c) = r.captures(&result.name) {
                        parse_capture!(let file: String = c);
                        parse_capture!(let line: u64 = c);

                        let real_line = location.line + line - 3;
                        let real_column = location.column + 4;
                        return Ok((file, real_line, real_column));
                    }
                    miette::bail!("Doctest title in unexpected format: {}", &result.name);
                })?;

            annotations.push(CheckRunOutputAnnotation {
                annotation_level: CheckRunOutputAnnotationLevel::Failure,
                path: location.file.clone(),
                start_line: real_line as u32,
                end_line: real_line as u32,
                start_column: Some(real_column as u32),
                end_column: None,
                message: format!(
                    r#"features: [{}]
    
cause:
{}

{}"#,
                    features.join(", "),
                    failure.panic_text.replace("\r\n", "\n").replace('\r', "\n"),
                    failure.stacktrace.replace("\r\n", "\n").replace('\r', "\n")
                ),
                title: Some(result.name.clone()),
                raw_details: Some(format!("{:#?}", result)),
            })
        }
    }

    let repo = std::env::var("GITHUB_REPOSITORY").expect("GITHUB_REPOSITORY env variable");
    let mut repo_split = repo.split('/');
    let owner = repo_split.next().expect("repo owner");
    let repo = repo_split.next().expect("repo");
    let sha = std::env::var("GITHUB_SHA").expect("GITHUB_SHA env variable");

    let checks = octocrab.checks(owner, repo);
    let annotations_count = annotations.len();
    if annotations.is_empty() {
        let output = CheckRunOutput {
            annotations,
            title: name.clone(),
            summary: format!("{} test failures", annotations_count),
            text: None,
            images: Vec::new(),
        };
        let _check_run = checks
            .create_check_run(name, sha)
            .output(output)
            .status(CheckRunStatus::Completed)
            .conclusion(CheckRunConclusion::Success)
            .completed_at(Utc::now())
            .send()
            .await
            .into_diagnostic()?;
    } else if annotations_count < 50 {
        let output = CheckRunOutput {
            annotations,
            title: name.clone(),
            summary: format!("{} test failures", annotations_count),
            text: None,
            images: Vec::new(),
        };
        let _check_run = checks
            .create_check_run(name, sha)
            .output(output)
            .status(CheckRunStatus::Completed)
            .conclusion(CheckRunConclusion::Failure)
            .completed_at(Utc::now())
            .send()
            .await
            .into_diagnostic()?;
    } else {
        todo!("report annotations in batches when > 50; API limitation")
    }
    // TODO: Check the return value from the GitHub API for errors and such.

    Ok(())
}

thread_local! {
    static DOCTEST_NAME_FILE_REGEX: Regex = Regex::new(r"(?P<file>.+?) - \(line (?P<line>\d+)\)").unwrap();
}
