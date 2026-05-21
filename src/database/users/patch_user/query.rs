use crate::database::users::patch_user::PatchUserQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn patch_user_query(
    view: PatchUserQueryView,
    pool: &PgPool,
) -> Result<(), DatabaseError> {
    let mut query_builder = sqlx::QueryBuilder::new("UPDATE users SET ");

    // On utilise un booléen pour savoir si on a déjà ajouté un champ
    let mut first = true;

    // Macro pour ajouter proprement chaque champ
    macro_rules! add_field {
        ($field_name:expr, $value:expr) => {
            if !first {
                query_builder.push(", ");
            }
            query_builder.push($field_name);
            query_builder.push(" = ");
            query_builder.push_bind($value);
            first = false;
        };
    }

    if let Some(first_name) = view.first_name() {
        add_field!("first_name", first_name);
    }
    if let Some(last_name) = view.last_name() {
        add_field!("last_name", last_name);
    }
    if let Some(email) = view.email() {
        add_field!("email", email);
    }
    if let Some(phone_number) = view.phone_number() {
        add_field!("phone_number", phone_number);
    }

    // Si aucun champ n'a été ajouté, on arrête tout
    if first {
        return Ok(());
    }

    // Ajout de la clause WHERE
    query_builder.push(" WHERE id = ");
    query_builder.push_bind(view.id() as i32);

    query_builder
        .build()
        .execute(pool)
        .await
        .map_err(DatabaseError::from)?;

    Ok(())
}
