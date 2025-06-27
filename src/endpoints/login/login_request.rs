use actix_web::{post, web, HttpResponse, Responder};

use super::super::super::database::db_interface::get_db_interface;
use super::login_view::LoginView;
use crate::database::queries_result_views::{
    get_boolean_from_query_result, get_result_from_query_result,
};
use crate::database::query_views::{LoginUserQueryView};

#[derive(Debug, Clone, PartialEq)]
enum LoginError {
    DatabaseError,
    // InvalidCredentials,
    // UserNotFound,
}

// impl std::fmt::Display for Loginrror {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//         }
//     }
// }

async fn login_user(login_view: &LoginView) -> Result<(), LoginError> {
    let view = LoginUserQueryView::new(
        login_view.email(),
        login_view.password(),
    );
    println!("Executing login query: {}", view);
    let db_guard = get_db_interface().lock().unwrap();
    let db_interface = match &*db_guard {
        Some(db) => db,
        None => {
            eprintln!("Database interface is not initialized.");
            return Err(LoginError::DatabaseError);
        }
    };
    let query_view = db_interface.execute_query(Box::new(view)).await;
    return Ok(());
    // match query_view {
    //     Ok(result) => match get_result_from_query_result(result.get_result()) {
    //         Ok(_) => Ok(()),
    //         Err(e) => {
    //             eprintln!("Error processing query result: {}", e);
    //             Err(Loginrror::DatabaseError)
    //         }
    //     },
    //     Err(e) => {
    //         eprintln!("Error executing query: {}", e);
    //         Err(Loginrror::DatabaseError)
    //     }
    // }
}

#[post("/login")]
async fn login(payload: web::Json<LoginView>) -> impl Responder {
    let login_view = payload.into_inner();
    match login_user(&login_view).await {
        Ok(_) => HttpResponse::Ok().body("User login successfully!"),
        Err(LoginError::DatabaseError) => {
            eprintln!("Database error occurred during login.");
            HttpResponse::InternalServerError().body("Internal server error.")
        }
    }
}
