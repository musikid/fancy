use crate::{error::Error, interface::*};

#[derive(Debug)]
pub struct State<'a> {
  pub config: String,
  pub proxy: Option<AsyncFancyProxy<'a>>,
  pub last_error: Option<Error>,
  pub proxy_state: ProxyState,
}

#[derive(Debug)]
pub enum ProxyState {
  Uninitialized,
  Initialized,
  Error(zbus::Error),
}

impl<'a> State<'a> {
  pub fn new() -> Self {
    State {
      proxy: None,
      config: String::new(),
      last_error: None,
      proxy_state: ProxyState::Uninitialized,
    }
  }

  pub fn set_connection_error(&mut self, proxy_err: zbus::Error) {
    self.proxy_state = ProxyState::Error(proxy_err);
  }

  pub fn _set_error(&mut self, err: Error) {
    self.last_error = Some(err);
  }

  pub fn set_proxy(&mut self, proxy: AsyncFancyProxy<'a>) {
    self.proxy = Some(proxy);
    self.proxy_state = ProxyState::Initialized;
  }
}
