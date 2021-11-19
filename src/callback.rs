use crate::handler::Handler;
use reqwest::{Client, Request};
use tracing::{trace, instrument};
use std::fmt::{self, Debug, Display};
use tokio::sync::mpsc::Receiver;

/// Represents the current calculation state.
///
/// Because this enum provides univeral `From` implementations a caller should never have to
/// directly deal with this enum themselves. This has been left in the documentation largely to
/// help with understanding compilation error messages.
#[derive(Debug)]
pub enum Indeterminate<I: Debug, C> {
    /// A parsed item.
    Item(I),
    /// A callback to be invoked.
    Callback(Callback<I, C>),
}

impl<I: Debug, C> From<Callback<I, C>> for Indeterminate<I, C> {
    fn from(callback: Callback<I, C>) -> Self {
        Self::Callback(callback)
    }
}

impl<I: Debug, C> From<I> for Indeterminate<I, C> {
    fn from(item: I) -> Self {
        Self::Item(item)
    }
}

/// Transforms a `Request` into `Indeterminate`s.
#[derive(Debug)]
pub struct Callback<I, C> {
    request: Request,
    // A Box to provide type eraser for the handler
    handler: Box<dyn Handler<I, C>>,
    context: C,
}

impl<I: Debug, C: Debug> Callback<I, C> {
    /// Construct a new `Callback` to be processed.
    ///
    /// # Arguments
    /// - `handler`: The function to process the generated `Response`.
    /// - `request`: Used to generate a `Response`.
    /// - `context`: User-defined metadata passed to the `handler` function.
    pub fn new<H>(handler: H, request: Request, context: C) -> Self
    where
        H: Handler<I, C> + 'static,
    {
        Self {
            handler: Box::new(handler),
            request,
            context,
        }
    }

    /// Returns the `Request` that will be processed by the callback execution.
    pub fn target(&self) -> &Request {
        &self.request
    }

    /// Execute the callback with the provided client and logger.
    #[instrument]
    pub(crate) async fn run(
        self,
        client: Client,
    ) -> Result<Receiver<Indeterminate<I, C>>, reqwest::Error> {
        trace!(request = ?self.request, "Executing request");
        let resp = client.execute(self.request).await?;
        trace!(response = ?resp, "Got response");
        let result = self.handler.handle(client, resp, self.context);
        Ok(result)
    }
}

impl<I, C> Display for Callback<I, C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} -> {}", self.handler, self.request.url())
    }
}
