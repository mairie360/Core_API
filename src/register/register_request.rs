use actix_web::{
    HttpResponse,
    post,
    Responder,
    web
};

use crate::database::query_views::DoesUserExistByEmailQueryView;
use super::super::database::db_interface::get_db_interface;
use super::register_view::RegisterView;
use crate::database::queries_result_views::get_boolean_from_query_result;

fn is_valid_email(email: String) -> bool {
    //Need to be more complex and based on requirements
    true
}

fn is_valid_password(password: String) -> bool {
    //Need to be more complex and based on requirements
    password.len() >= 8
}

async fn already_exists(register_view: &RegisterView) -> bool {
    let view = DoesUserExistByEmailQueryView::new(register_view.email());
    println!("db view: {}", view);
    let query_view= get_db_interface().lock().unwrap().execute_query(Box::new(view)).await;
    match query_view {
        Ok(result) => {
            println!("Query result: {}", result.get_result());
            get_boolean_from_query_result(result.get_result())
        },
        Err(e) => {
            println!("Error executing query: {}", e);
            true
        }
    }
    // match query_view {
    //     Ok(result) => false,
    //     Err(_) => false
    // }
}

async fn can_be_registered(register_view: &RegisterView) -> Result<(), String> {
    if !is_valid_email(register_view.email()) {
        return Err("Invalid email format".to_string());
    }
    if already_exists(&register_view).await {
        return Err("User already exists".to_string());
    }
    if !is_valid_password(register_view.password()) {
        return Err("Password must be at least 8 characters long".to_string());
    }
    Ok(())
}

async fn register_user(register_view: &RegisterView) -> Result<(), String> {
    match can_be_registered(register_view).await {
        Ok(_) => {
            //Need to be implemented after db link
            Ok(())
        },
        Err(e) => Err(e),
    }
}

#[post("/register")]
async fn register(payload: web::Json<RegisterView>) -> impl Responder {
    let register_view = payload.into_inner();
    println!("{}", register_view);
    match register_user(&register_view).await {
        Ok(_) => {
            return HttpResponse::Created().body("User registered successfully!");
        },
        Err(e) => {
            return HttpResponse::BadRequest().body(format!("Error: {}", e));
        }
    }
}