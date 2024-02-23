#[derive(Debug, thiserror::Error)]
pub enum Error<H> {
    #[error(transparent)]
    Handler(H),
}
