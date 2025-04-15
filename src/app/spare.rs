use api::*;

use super::AppState;

pub trait SpareAPI {
    async fn spare_questionaire(
        &self,
        req: SpareQuestionaireRequest,
        auth: Auth,
    ) -> SpareQuestionaireResponse;
    async fn spare_return(&self, req: SpareReturnRequest, auth: Auth) -> SpareReturnResponse;
    async fn spare_take(&self, req: SpareTakeRequest, auth: Auth) -> SpareTakeResponse;
    async fn spare_list(&self, req: SpareListRequest, auth: Auth) -> SpareListResponse;
}

#[allow(unused)]
impl SpareAPI for AppState {
    async fn spare_questionaire(
        &self,
        req: SpareQuestionaireRequest,
        auth: Auth,
    ) -> SpareQuestionaireResponse {
        todo!()
    }

    async fn spare_return(&self, req: SpareReturnRequest, auth: Auth) -> SpareReturnResponse {
        todo!()
    }

    async fn spare_take(&self, req: SpareTakeRequest, auth: Auth) -> SpareTakeResponse {
        todo!()
    }

    async fn spare_list(&self, req: SpareListRequest, auth: Auth) -> SpareListResponse {
        todo!()
    }
}

#[cfg(test)]
#[allow(unused)]
mod test{
    use sqlx::SqlitePool;

    use crate::app::test::TestApp;

    use super::*;

    #[sqlx::test]
    #[ignore]
    async fn test_spare_questionaire(pool: SqlitePool){
        // Create a new test app instance
        let app = TestApp::new(pool);
        
        todo!()
    }

    // todo
}