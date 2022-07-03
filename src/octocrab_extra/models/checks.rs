use octocrab::models::pulls::PullRequest;
use octocrab::models::App;
use serde_json::Value;

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CheckRunStatus {
    Queued,
    InProgress,
    Completed,
}

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CheckRunConclusion {
    ActionRequired,
    Cancelled,
    Failure,
    Neutral,
    Success,
    Skipped,
    Stale,
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct CheckRunOutputArgument {
    pub title: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<CheckRunAnnotation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<CheckRunImage>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct CheckRunAnnotation {
    /// The path of the file to add an annotation to. For example, assets/css/main.css.
    pub path: String,
    /// The start line of the annotation.
    pub start_line: u64,
    /// The end line of the annotation.
    pub end_line: u64,
    /// The start column of the annotation. Annotations only support
    /// `start_column` and `end_column` on the same line.
    /// Omit this parameter if `start_line` and `end_line` have different
    /// values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_column: Option<u64>,
    /// The end column of the annotation. Annotations only support
    /// `start_column` and `end_column` on the same line.
    /// Omit this parameter if `start_line` and `end_line` have different
    /// values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<u64>,
    /// The level of the annotation.
    pub annotation_level: AnnotationLevel,
    /// A short description of the feedback for these lines of code. The
    /// maximum size is 64 KB.
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The title that represents the annotation. The maximum size is 255
    /// characters.
    pub title: Option<String>,
    /// Details about this annotation. The maximum size is 64 KB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_details: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct CheckRunImage {
    /// The alternative text for the image.
    pub alt: String,
    /// The full URL of the image.
    pub image_url: String,
    /// A short image description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct CheckRunAction {
    /// The text to be displayed on a button in the web UI. The maximum size
    /// is 20 characters.
    pub label: String,
    /// A short explanation of what this action would do. The maximum size is
    /// 40 characters.
    pub description: String,
    /// A reference for the action on the integrator's system. The maximum
    /// size is 20 characters.
    pub identifier: String,
}

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AnnotationLevel {
    Notice,
    Warning,
    Failure,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct CheckRunOutputResponse {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub text: Option<String>,
    pub annotations_count: u64,
    pub annotations_url: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct CheckRunCheckSuite {
    pub id: u64,
}

/// A check performed on the code of a given code change
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct CheckRun {
    /// The id of the check.
    pub id: u64,
    /// The SHA of the commit that is being checked.
    pub head_sha: String,
    pub node_id: String,
    pub external_id: Option<String>,
    pub url: String,
    pub html_url: Option<String>,
    pub details_url: Option<String>,
    /// The phase of the lifecycle that the check is currently in.
    pub status: CheckRunStatus,
    pub conclusion: Option<CheckRunConclusion>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub output: CheckRunOutputResponse,
    pub name: String,
    pub check_suite: Option<CheckRunCheckSuite>,
    pub app: Option<App>,
    pub pull_requests: Vec<PullRequest>,
    pub deployment: Value,
}
