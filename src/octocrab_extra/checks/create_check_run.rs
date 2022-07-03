use crate::octocrab_extra::models::checks::{
    CheckRun, CheckRunAction, CheckRunConclusion, CheckRunOutputArgument, CheckRunStatus,
};

#[derive(serde::Serialize)]
pub struct CreateCheckRunBuilder<'octo, 'r> {
    #[serde(skip)]
    handler: &'r super::CheckHandler<'octo>,
    name: String,
    head_sha: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<CheckRunStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    conclusion: Option<CheckRunConclusion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<CheckRunOutputArgument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actions: Option<Vec<CheckRunAction>>,
}

#[allow(dead_code)]
impl<'octo, 'r> CreateCheckRunBuilder<'octo, 'r> {
    pub fn new(handler: &'r super::CheckHandler<'octo>, name: String, head_sha: String) -> Self {
        Self {
            handler,
            name,
            head_sha,
            details_url: None,
            external_id: None,
            status: None,
            started_at: None,
            conclusion: None,
            completed_at: None,
            output: None,
            actions: None,
        }
    }

    /// The URL of the integrator's site that has the full details of the
    /// check. If the integrator does not provide this, then the homepage of
    /// the GitHub app is used.
    pub fn details_url(mut self, details_url: impl Into<String>) -> Self {
        self.details_url = Some(details_url.into());
        self
    }

    /// A reference for the run on the integrator's system.
    pub fn external_id(mut self, external_id: impl Into<String>) -> Self {
        self.external_id = Some(external_id.into());
        self
    }

    /// The current status.
    /// Default: [`CheckRunStatus::Queued`]
    pub fn status(mut self, status: impl Into<CheckRunStatus>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// The time that the check run began.
    pub fn started_at(mut self, started_at: impl Into<chrono::DateTime<chrono::Utc>>) -> Self {
        self.started_at = Some(started_at.into());
        self
    }

    /// The final conclusion of the check. **Required if you provide
    /// [`completed_at`](Self::completed_at) or a status of
    /// [`CheckRunStatus::Completed`]**.
    /// Note: Providing conclusion will automatically set the status parameter
    /// to completed. You cannot change a check run conclusion to stale, only
    /// GitHub can set this.
    pub fn conclusion(mut self, conclusion: impl Into<CheckRunConclusion>) -> Self {
        self.conclusion = Some(conclusion.into());
        self
    }

    /// The time that the check completed.
    pub fn completed_at(mut self, completed_at: impl Into<chrono::DateTime<chrono::Utc>>) -> Self {
        self.completed_at = Some(completed_at.into());
        self
    }

    /// Check runs can accept a variety of data in the `output` object,
    /// including a `title` and `summary` and can optionally provide
    /// descriptive details about the run.
    pub fn output(mut self, output: impl Into<CheckRunOutputArgument>) -> Self {
        self.output = Some(output.into());
        self
    }

    /// Displays a button on GitHub that can be clicked to alert your app to
    /// do additional tasks. For example, a code linting app can display a
    /// button that automatically fixes detected errors. The button created in
    /// this object is displayed after the check run completes. When a user
    /// clicks the button, GitHub sends the `check_run.requested_action`
    /// webhook to your app. Each action includes a `label`, `identifier`
    /// and `description`. A maximum of three actions are accepted.
    pub fn actions(mut self, actions: impl Into<Vec<CheckRunAction>>) -> Self {
        self.actions = Some(actions.into());
        self
    }

    /// Send the actual request.
    pub async fn send(self) -> octocrab::Result<CheckRun> {
        let route = format!(
            "repos/{owner}/{repo}/check-runs",
            owner = self.handler.owner,
            repo = self.handler.repo,
        );

        self.handler.crab.post(route, Some(&self)).await
    }
}
