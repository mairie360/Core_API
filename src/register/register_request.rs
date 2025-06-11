use actix_web::{
    HttpResponse,
    post,
    Responder,
    web
};

use super::register_view::RegisterView;

fn is_valid_email(email: &str) -> bool {
    //Need to be more complex and based on requirements
    true
}

fn is_valid_password(password: &str) -> bool {
    //Need to be more complex and based on requirements
    password.len() >= 8
}

fn already_exists(register_view: &RegisterView) -> bool {
    //Need to be implemented after db link
    false
}

fn can_be_registered(register_view: &RegisterView) -> Result<(), String> {
    if !is_valid_email(register_view.email()) {
        return Err("Invalid email format".to_string());
    }
    if already_exists(&register_view) {
        return Err("User already exists".to_string());
    }
    if !is_valid_password(register_view.password()) {
        return Err("Password must be at least 8 characters long".to_string());
    }
    Ok(())
}

fn register_user(register_view: &RegisterView) -> Result<(), String> {
    match can_be_registered(register_view) {
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
    match register_user(&register_view) {
        Ok(_) => {
            return HttpResponse::Ok().body("User registered successfully!");
        },
        Err(e) => {
            return HttpResponse::BadRequest().body(format!("Error: {}", e));
        }
    }
}