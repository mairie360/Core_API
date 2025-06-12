use crate::database::query_views::DoesUserExistByEmailQueryView;
use crate::database::queries_result_views::DoesUserExistByEmailQueryResultView;

pub fn does_user_exist_by_email(query: Box<DoesUserExistByEmailQueryView>) -> Box<DoesUserExistByEmailQueryResultView> {
    println!("Executing does_user_exist_by_email query with email: {}", query.get_email());
    match query.get_email().is_empty() {
        true => {
            Box::new(DoesUserExistByEmailQueryResultView::new(false))
        },
        false => {
            Box::new(DoesUserExistByEmailQueryResultView::new(false))
        }
    }
}