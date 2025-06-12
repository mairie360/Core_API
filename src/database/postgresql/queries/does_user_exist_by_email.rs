use crate::database::queries_result_views::DoesUserExistByEmailQueryResultView;
use crate::database::db_interface::{DatabaseQueryView, QueryResultView};

pub async fn does_user_exist_by_email(query: Box<dyn DatabaseQueryView>) -> Result<Box<dyn QueryResultView>, String> {
    println!("Executing request: {}", query.get_request());
    Ok(Box::new(DoesUserExistByEmailQueryResultView::new(false)))
    // println!("Executing does_user_exist_by_email query with email: {}", query.get_email());
    // match query.get_email().is_empty() {
    //     true => {
    //         Box::new(DoesUserExistByEmailQueryResultView::new(false))
    //     },
    //     false => {
    //         Box::new(DoesUserExistByEmailQueryResultView::new(false))
    //     }
    // }
}