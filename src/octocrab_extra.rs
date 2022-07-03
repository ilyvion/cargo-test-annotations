use octocrab::Octocrab;

pub mod models{
    pub mod checks;
}
mod checks;

pub trait OctocrabExt {
    fn checks(&self, owner: impl Into<String>, repo: impl Into<String>) -> checks::CheckHandler<'_>;
}

impl OctocrabExt for Octocrab {
    fn checks(&self, owner: impl Into<String>, repo: impl Into<String>) -> checks::CheckHandler<'_> {
        checks::CheckHandler::new(self, owner.into(), repo.into())
    }
}

// pub fn checks(
//     &self,
//     owner: impl Into<String>,
//     repo: impl Into<String>,
// ) -> issues::IssueHandler {
//     issues::IssueHandler::new(self, owner.into(), repo.into())
// }
