#![feature(const_trait_impl)]

pub mod user;
use axum::{
  http::{Request, StatusCode},
  middleware::Next,
  response::Response,
  Extension,
};
use cookie::Cookie;
use gid::gid;
use trt::TRT;
use x0::{fred::interfaces::HashesInterface, R};
use xxai::unzip_u64;
use xxhash_rust::xxh3::xxh3_64;

#[derive(Debug, Clone, Copy)]
pub struct _Client {
  pub id: u64,
  _user_id: Option<u64>,
}

pub type Client = Extension<_Client>;

static mut SK: [u8; 32] = [0; 32];

const MAX_INTERVAL: u64 = 41;
const TOKEN_LEN: usize = 8;

/*
   cookie 中的 day 每10天为一个周期，超过41个周期没访问就认为无效, BASE是为了防止数字过大
   https://chromestatus.com/feature/4887741241229312
   When cookies are set with an explicit Expires/Max-Age attribute the value will now be capped to no more than 400 day10s

*/
const BASE: u64 = 4096;

fn day10() -> u64 {
  (xxai::now() / (86400 * 10)) % BASE
}

#[ctor::ctor]
fn init() {
  TRT.block_on(async move {
    let redis = R.0.force().await;
    let conf = &b"conf"[..];
    let key = &b"SK"[..];
    let sk: Option<Vec<u8>> = redis.hget(conf, key).await.unwrap();
    let len = unsafe { SK.len() };
    if let Some(sk) = sk {
      if sk.len() == len {
        unsafe { SK = sk.try_into().unwrap() };
        return;
      }
    }
    use xxai::random_bytes;
    let sk = &random_bytes(len)[..];
    redis.hset::<(), _, _>(conf, vec![(key, sk)]).await.unwrap();
    unsafe { SK = sk.try_into().unwrap() };
  })
}

#[derive(Debug)]
pub enum ClientState {
  Ok(u64),
  Renew(u64),
  None,
}

fn client_by_cookie(token: &str) -> ClientState {
  if let Ok(c) = xxai::cookie_decode(token) {
    if c.len() >= TOKEN_LEN {
      let client = &c[TOKEN_LEN..];
      if xxh3_64(&[unsafe { &SK }, client].concat())
        == u64::from_le_bytes(c[..TOKEN_LEN].try_into().unwrap())
      {
        let li = unzip_u64(client);
        if li.len() == 2 {
          let [pre_day10, client]: [u64; 2] = li.try_into().unwrap();

          let now = day10();
          if pre_day10 != now {
            // 因为都是无符号类型，要避免减法出现负数
            if pre_day10 > now {
              if pre_day10 < BASE && (now + BASE - pre_day10) < MAX_INTERVAL {
                return ClientState::Renew(client);
              }
            } else if (now - pre_day10) < MAX_INTERVAL {
              // renew
              return ClientState::Renew(client);
            }
          } else {
            return ClientState::Ok(client);
          }
        }
      }
    }
  }
  ClientState::None
}

fn header_get<B>(req: &Request<B>, key: impl AsRef<str>) -> Option<&str> {
  req
    .headers()
    .get(key.as_ref())
    .and_then(|header| header.to_str().ok())
}

pub async fn client<B>(req: Request<B>, next: Next<B>) -> Result<Response, StatusCode> {
  _client(req, next).await
}

pub async fn _client<B>(mut req: Request<B>, next: Next<B>) -> Result<Response, StatusCode> {
  let mut client = 0;

  if let Some(cookie) = header_get(&req, http::header::COOKIE) {
    for cookie in Cookie::split_parse(cookie).flatten() {
      if cookie.name() == "I" {
        match client_by_cookie(cookie.value()) {
          ClientState::Renew(id) => {
            client = id;
          }
          ClientState::Ok(id) => {
            req.extensions_mut().insert(_Client {
              id,
              _user_id: Some(0),
            });
            return Ok(next.run(req).await);
          }
          _ => {}
        }
        break;
      }
    }
  }

  let host = xxai::tld(header_get(&req, http::header::HOST).unwrap());
  let _user_id = if client == 0 {
    client = gid!(client);
    Some(0)
  } else {
    None
  };
  req.extensions_mut().insert(_Client {
    id: client,
    _user_id,
  });

  let mut r = next.run(req).await;

  let t = &xxai::zip_u64([day10(), client])[..];
  let cookie =
    xxai::cookie_encode([&xxh3_64(&[unsafe { &SK }, t].concat()).to_le_bytes()[..], t].concat());
  r.headers_mut().insert(
    http::header::SET_COOKIE,
    format!("I={cookie};max-age=99999999;domain={host};path=/;HttpOnly;SameSite=None;Secure")
      .parse()
      .unwrap(),
  );
  Ok(r)
}
