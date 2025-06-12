#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QUERY {
    DoesUserExistByEmail,
    UnknownQuery,
}