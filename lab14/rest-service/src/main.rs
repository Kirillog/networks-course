use axum::{
    body::Bytes,
    extract::{ConnectInfo, FromRef, Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, Message,
    SmtpTransport, Transport,
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::NoneAsEmptyString;
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
};
use std::{sync::Arc, time::Duration};
use tokio::{sync::Mutex, task, time};

type Uuid = u32;

#[derive(Serialize, Deserialize, Clone)]
struct Product {
    id: Uuid,
    name: String,
    description: String,
    image: String,
}

#[derive(Serialize, Deserialize)]
struct UserCredentials {
    email: String,
    password: String,
}

#[derive(Clone)]
struct User {
    email: String,
    password: String,
    ip: IpAddr,
}

#[derive(Clone)]
struct ConnectionInfo {
    last_access: i64,
    message_sended: bool,
}

#[derive(Clone)]
struct ServerState {
    public_products: HashMap<Uuid, Product>,
    images: HashMap<String, Bytes>,
    users: Vec<User>,
    connect_info: Vec<ConnectionInfo>,
    connected_users: HashMap<uuid::Uuid, usize>,
    private_products: Vec<HashMap<Uuid, Product>>,
    max_id: Uuid,
}

const NUM_DELAY_SECONDS: u64 = 10;
const MAIL_LOGIN: &'static str = ""; // Service email
const MAIL_PASSWORD: &'static str = ""; // Service external password

impl ServerState {
    fn images_n_products(&mut self) -> (&mut HashMap<Uuid, Product>, &mut HashMap<String, Bytes>) {
        (&mut self.public_products, &mut self.images)
    }

    fn users_n_info(&mut self) -> (&mut Vec<User>, &mut Vec<ConnectionInfo>) {
        (&mut self.users, &mut self.connect_info)
    }
}

#[derive(Clone, FromRef)]
struct AppState {
    data: Arc<Mutex<ServerState>>,
}

fn send_message(user: &User) {
    let email = Message::builder()
        .from(MAIL_LOGIN.parse().unwrap())
        .to(user.email.parse().unwrap())
        .subject("Welcome")
        .header(ContentType::TEXT_PLAIN)
        .body(String::from(
            "Рады приветствовать вас на нашем сервисе вновь!",
        ))
        .unwrap();

    let creds = Credentials::new(MAIL_LOGIN.to_owned(), MAIL_PASSWORD.to_owned());

    // Open a remote connection to mail
    let mailer = SmtpTransport::relay("smtp.mail.ru")
        .unwrap()
        .credentials(creds)
        .build();

    // Send the email
    match mailer.send(&email) {
        Ok(_) => println!("Email sent successfully!"),
        Err(e) => panic!("Could not send email: {e:?}"),
    }
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let state = AppState {
        data: Arc::new(Mutex::new(ServerState {
            public_products: HashMap::new(),
            images: HashMap::new(),
            max_id: 0,
            users: Vec::new(),
            connect_info: Vec::new(),
            private_products: Vec::new(),
            connected_users: HashMap::new(),
        })),
    };

    let cloned_state = state.clone();

    task::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(NUM_DELAY_SECONDS));
        loop {
            interval.tick().await;
            {
                let mut data = cloned_state.data.lock().await;
                let cur_time = Utc::now();
                let (users, connect_info) = data.users_n_info();
                connect_info
                    .iter_mut()
                    .enumerate()
                    .filter(|(_, info)| {
                        let st = DateTime::from_timestamp(info.last_access, 0).unwrap();
                        !info.message_sended
                            && (cur_time - st).num_seconds() > NUM_DELAY_SECONDS as i64
                    })
                    .for_each(|(index, info)| {
                        info.message_sended = true;
                        send_message(&users[index])
                    });
            }
        }
    });

    // build our application with a route
    let app = Router::new()
        .route("/products", get(get_products))
        .route("/product", post(create_product))
        .route("/product/:id", get(get_product))
        .route("/product/:id", put(change_product))
        .route("/product/:id", delete(delete_product))
        .route("/product/:id/image", post(post_product_image))
        .route("/product/:id/image", get(get_product_image))
        .route("/user/signup", post(user_signup))
        .route("/user/signin", post(user_signin))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("localhost:8080")
        .await
        .unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn get_product(
    Path(id): Path<Uuid>,
    Query(token): Query<Token>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> Response {
    let mut data = state.data.lock().await;

    let index = if token.token.is_some() {
        let t = token.token.unwrap();
        data.connected_users.get(&t).map(|ind| *ind)
    } else {
        data.users.iter().position(|user| user.ip == addr.ip())
    };
    if let Some(index) = index {
        data.connect_info[index].last_access = Utc::now().timestamp();
        data.connect_info[index].message_sended = false;
    }

    let value = match data.public_products.get(&id) {
        Some(value) => value.clone(),
        None if token.token.is_some() => {
            let t = token.token.unwrap();
            let index = match data.connected_users.get(&t) {
                Some(index) => *index,
                None => {
                    return StatusCode::BAD_REQUEST.into_response();
                }
            };
            match data.private_products[index].get(&id) {
                Some(value) => value.clone(),
                _ => {
                    return StatusCode::BAD_REQUEST.into_response();
                }
            }
        }
        _ => {
            return StatusCode::BAD_REQUEST.into_response();
        }
    };
    (StatusCode::OK, Json(value)).into_response()
}

#[derive(Deserialize)]
struct CreateProduct {
    name: String,
    description: String,
}

async fn create_product(
    State(state): State<AppState>,
    Query(token): Query<Token>,
    Json(payload): Json<CreateProduct>,
) -> Response {
    let mut data = state.data.lock().await;
    let uuid = data.max_id;
    data.max_id += 1;
    let product = Product {
        id: uuid.clone(),
        name: payload.name,
        description: payload.description,
        image: String::from(""),
    };
    match token.token {
        None => {
            data.public_products.insert(uuid, product.clone());
        }
        Some(t) => {
            let index = match data.connected_users.get(&t) {
                Some(index) => *index,
                None => {
                    return StatusCode::BAD_REQUEST.into_response();
                }
            };
            data.private_products[index].insert(uuid, product.clone());
        }
    }
    (StatusCode::OK, Json(product)).into_response()
}

fn consume_inspect<T, F>(opt: Option<T>, f: F)
where
    F: FnOnce(T),
{
    match opt {
        Some(value) => f(value),
        None => {}
    }
}

#[derive(Deserialize)]
struct ChangeProduct {
    name: Option<String>,
    description: Option<String>,
}

async fn change_product(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(token): Query<Token>,
    Json(payload): Json<ChangeProduct>,
) -> Response {
    let mut data = state.data.lock().await;
    let value = match data.public_products.get_mut(&id) {
        Some(value) => value,
        None if token.token.is_some() => {
            let t = token.token.unwrap();
            let index = match data.connected_users.get(&t) {
                Some(index) => *index,
                None => {
                    return StatusCode::BAD_REQUEST.into_response();
                }
            };
            match data.private_products[index].get_mut(&id) {
                Some(value) => value,
                _ => {
                    return StatusCode::BAD_REQUEST.into_response();
                }
            }
        }
        _ => {
            return StatusCode::BAD_REQUEST.into_response();
        }
    };
    let ChangeProduct { name, description } = payload;
    consume_inspect(name, |name| value.name = name);
    consume_inspect(description, |description| value.description = description);
    (StatusCode::OK, Json(value)).into_response()
}

async fn delete_product(
    Path(id): Path<Uuid>,
    Query(token): Query<Token>,
    State(state): State<AppState>,
) -> Response {
    let mut data = state.data.lock().await;
    let value = match data.public_products.remove(&id) {
        Some(value) => value,
        None if token.token.is_some() => {
            let t = token.token.unwrap();
            let index = match data.connected_users.get(&t) {
                Some(index) => *index,
                None => {
                    return StatusCode::BAD_REQUEST.into_response();
                }
            };
            match data.private_products[index].remove(&id) {
                Some(value) => value,
                _ => {
                    return StatusCode::BAD_REQUEST.into_response();
                }
            }
        }
        _ => {
            return StatusCode::BAD_REQUEST.into_response();
        }
    };
    (StatusCode::OK, Json(value)).into_response()
}

async fn get_products(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(token): Query<Token>,
) -> Response {
    let mut data = state.data.lock().await;

    let index = if token.token.is_some() {
        let t = token.token.unwrap();
        data.connected_users.get(&t).map(|ind| *ind)
    } else {
        data.users.iter().position(|user| user.ip == addr.ip())
    };
    if let Some(index) = index {
        data.connect_info[index].last_access = Utc::now().timestamp();
        data.connect_info[index].message_sended = false;
    }

    let mut vec = data
        .public_products
        .values()
        .map(|ref_value| ref_value.clone())
        .collect::<Vec<_>>();
    if token.token.is_some() && data.connected_users.get(&token.token.unwrap()).is_some() {
        let index = *data.connected_users.get(&token.token.unwrap()).unwrap();
        vec.extend(
            data.private_products[index]
                .values()
                .map(|ref_value| ref_value.clone()),
        );
    }
    vec.sort_by_key(|product| product.id);
    (StatusCode::OK, Json(vec)).into_response()
}

async fn post_product_image(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Response {
    let mut serv_data = state.data.lock().await;

    let (products, images) = serv_data.images_n_products();

    let product = match products.get_mut(&id) {
        Some(value) => value,
        None => {
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    while let Some(field) = multipart.next_field().await.unwrap() {
        let file_name = field.file_name().unwrap().to_string();
        let content_type = field.content_type().unwrap().to_string();
        if content_type != "image/png" {
            return StatusCode::BAD_REQUEST.into_response();
        }
        let data = field.bytes().await.unwrap();
        product.image = file_name.clone();
        images.insert(file_name, data);
    }
    StatusCode::OK.into_response()
}

async fn get_product_image(Path(id): Path<Uuid>, State(state): State<AppState>) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "image/png".parse().unwrap());
    let data = state.data.lock().await;
    match data.public_products.get(&id) {
        Some(value) => match data.images.get(&value.image) {
            Some(image) if !image.is_empty() => {
                (StatusCode::OK, headers, image.clone()).into_response()
            }
            _ => StatusCode::BAD_REQUEST.into_response(),
        },
        None => StatusCode::BAD_REQUEST.into_response(),
    }
}

async fn user_signup(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<UserCredentials>,
) -> Response {
    let mut data = state.data.lock().await;
    if data
        .users
        .iter()
        .find(|user| user.email == payload.email)
        .is_some()
    {
        StatusCode::BAD_REQUEST.into_response()
    } else {
        data.users.push(User {
            email: payload.email,
            password: payload.password,
            ip: addr.ip(),
        });
        data.connect_info.push(ConnectionInfo {
            last_access: Utc::now().timestamp(),
            message_sended: false,
        });
        data.private_products.push(HashMap::new());
        StatusCode::OK.into_response()
    }
}

#[serde_as]
#[derive(Deserialize, Serialize)]
struct Token {
    #[serde_as(as = "NoneAsEmptyString")]
    token: Option<uuid::Uuid>,
}

async fn user_signin(
    State(state): State<AppState>,
    Json(payload): Json<UserCredentials>,
) -> Response {
    let mut data = state.data.lock().await;
    match data
        .users
        .iter()
        .position(|user| user.email == payload.email && user.password == payload.password)
    {
        None => StatusCode::BAD_REQUEST.into_response(),
        Some(index) => {
            let token = uuid::Uuid::new_v4();
            data.connected_users.insert(token, index);
            (StatusCode::OK, Json(Token { token: Some(token) })).into_response()
        }
    }
}
