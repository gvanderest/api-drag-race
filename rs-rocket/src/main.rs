#[macro_use]
extern crate rocket;

use std::str::FromStr;

use mongodb::{
    bson::{doc, oid::ObjectId, to_bson},
    options::ClientOptions,
    Client, Collection,
};
use optfield::optfield;
use rocket::{
    serde::{json::Json, Deserialize, Serialize},
    State,
};

#[derive(Responder)]
enum CreateAccountResponse {
    #[response(status = 200)]
    Success(Json<Account>),
}

#[post("/accounts", data = "<account>")]
async fn create_account(account: Json<Account>, db: &State<Database>) -> CreateAccountResponse {
    let mut account = account.into_inner();

    account.id = None;

    let result = db.accounts.insert_one(&account, None).await.unwrap();

    CreateAccountResponse::Success(Json(Account {
        id: Some(result.inserted_id.as_object_id().unwrap()),
        ..account
    }))
}

#[derive(Serialize, Deserialize)]
struct ErrorResponse {
    error_code: String,
    message: String,
}

#[derive(Responder)]
enum ReadAccountResponse {
    #[response(status = 200)]
    Success(Json<Account>),
    #[response(status = 404)]
    NotFound(Json<ErrorResponse>),
}

#[get("/account/<id>")]
async fn read_account(id: &str, db: &State<Database>) -> ReadAccountResponse {
    let id_object_result = ObjectId::from_str(id);

    match id_object_result {
        Err(_) => ReadAccountResponse::NotFound(Json(ErrorResponse {
            error_code: "NOT_FOUND".to_string(),
            message: "The requested account could not be found".to_string(),
        })),
        Ok(id_object) => {
            let filter = doc! { "_id": id_object };
            let maybe_account = db.accounts.find_one(filter, None).await.unwrap();

            ReadAccountResponse::Success(Json(maybe_account.unwrap()))
        }
    }
}

#[derive(Responder)]
enum UpdateAccountResponse {
    #[response(status = 200, content_type = "application/json")]
    Success(Json<Account>),
    #[response(status = 400)]
    InvalidRequest(Json<ErrorResponse>),
    #[response(status = 404)]
    NotFound(Json<ErrorResponse>),
}

#[put("/account/<id>", data = "<json_account>")]
async fn update_account(
    id: &str,
    json_account: Json<UpdateAccount>,
    db: &State<Database>,
) -> UpdateAccountResponse {
    let id_object_result = ObjectId::from_str(id);

    let account = json_account.into_inner();

    if account.id.is_some() {
        return UpdateAccountResponse::InvalidRequest(Json(ErrorResponse {
            error_code: "INVALID_REQUEST".to_string(),
            message: "The ID cannot be provided".to_string(),
        }));
    }

    match id_object_result {
        Err(_) => UpdateAccountResponse::NotFound(Json(ErrorResponse {
            error_code: "NOT_FOUND".to_string(),
            message: "The requested account could not be found".to_string(),
        })),
        Ok(id_object) => {
            let filter = doc! { "_id": id_object };

            {
                let modifications = to_bson(&account).unwrap();
                db.accounts
                    .update_one(filter, doc! {"$set": modifications }, None)
                    .await
                    .unwrap();
            }

            let read_response = read_account(id, db).await;

            match read_response {
                ReadAccountResponse::Success(acc) => UpdateAccountResponse::Success(acc),
                ReadAccountResponse::NotFound(err) => UpdateAccountResponse::NotFound(err),
            }
        }
    }
}

#[optfield(UpdateAccount, attrs, field_attrs = (serde(skip_serializing_if = "Option::is_none")))]
#[derive(Debug, Serialize, Deserialize)]
struct Account {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    email: String,
    first_name: String,
    last_name: String,
}

struct Database {
    accounts: Collection<Account>,
}

impl Database {
    async fn init() -> Self {
        let client = Client::with_options(
            ClientOptions::builder()
                .retry_writes(false)
                .retry_reads(false)
                .build(),
        )
        .unwrap();
        let db = client.database("Example");

        let accounts: Collection<Account> = db.collection("Accounts");

        Self { accounts }
    }
}

#[launch]
async fn rocket() -> _ {
    let db = Database::init().await;

    rocket::build()
        .manage(db)
        .mount("/", routes![create_account, read_account, update_account])
}
