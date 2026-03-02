use crate::database::queries_result_views::LoginUserQueryResultView;
use crate::database::query_views::LoginUserQueryView;
use async_trait::async_trait;
use mairie360_api_lib::database::db_interface::{DatabaseQueryView, Query};
use mairie360_api_lib::database::errors::DatabaseError;
use mairie360_api_lib::database::queries::QueryError;
use tokio_postgres::Client;

pub struct LoginUserQuery {
    view: LoginUserQueryView,
}

impl LoginUserQuery {
    pub fn new(email: &str, password: &str) -> Self {
        Self {
            view: LoginUserQueryView::new(email.to_string(), password.to_string()),
        }
    }
}

#[async_trait]
impl Query for LoginUserQuery {
    type Output = LoginUserQueryResultView;

    async fn execute(&self, client: &Client) -> Result<Self::Output, DatabaseError> {
        // 1. On cherche l'utilisateur par email uniquement
        let email = self.view.get_email();
        let result = client
            .query_opt(self.view.get_request().as_str(), &[&email])
            .await;

        match result {
            Ok(Some(row)) => {
                let user_id: i32 = row
                    .try_get("id")
                    .map_err(|e| QueryError::MappingError(e.to_string()))?;

                let db_password: String = row
                    .try_get::<&str, String>("password")
                    .map_err(|e| QueryError::MappingError(e.to_string()))?
                    .trim_end()
                    .to_string();

                // 2. Comparaison du mot de passe
                if db_password == *self.view.get_password() {
                    Ok(LoginUserQueryResultView::new(user_id as u64))
                } else {
                    // Email trouvé, mais mot de passe incorrect
                    Err(QueryError::InvalidPassword(self.view.get_email().clone()).into())
                }
            }
            Ok(None) => {
                // Aucune ligne renvoyée : l'email n'existe pas en base
                Err(QueryError::EmailNotFound(self.view.get_email().clone()).into())
            }
            Err(e) => Err(QueryError::from(e).into()),
        }
    }
}
