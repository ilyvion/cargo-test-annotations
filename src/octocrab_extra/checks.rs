use octocrab::Octocrab;

mod create_check_run;

pub struct CheckHandler<'octo> {
    _crab: &'octo Octocrab,
    owner: String,
    repo: String,
}

impl<'octo> CheckHandler<'octo> {
    pub(crate) fn new(crab: &'octo Octocrab, owner: String, repo: String) -> Self {
        Self {
            _crab: crab,
            owner,
            repo,
        }
    }

    pub fn create_check_run(
        &self,
        name: String,
        head_sha: String,
    ) -> create_check_run::CreateCheckRunBuilder<'_, '_> {
        create_check_run::CreateCheckRunBuilder::new(self, name, head_sha)
    }
}
