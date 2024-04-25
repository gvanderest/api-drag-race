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

#[post("/accounts", data = "<account>")]
async fn create_account(account: Json<Account>, db: &State<Database>) -> Option<Json<Account>> {
    let mut account = account.into_inner();

    account.id = None;

    let result = db.accounts.insert_one(&account, None).await.unwrap();

    Some(Json(Account {
        id: Some(result.inserted_id.as_object_id().unwrap()),
        ..account
    }))
}

#[get("/account/<id>")]
async fn read_account(id: &str, db: &State<Database>) -> Option<Json<Account>> {
    let id_object_result = ObjectId::from_str(id);

    match id_object_result {
        Err(_) => None,
        Ok(id_object) => {
            let filter = doc! { "_id": id_object };
            let maybe_account = db.accounts.find_one(filter, None).await.unwrap();

            maybe_account.map(Json)
        }
    }
}

#[put("/account/<id>", data = "<json_account>")]
async fn update_account(
    id: &str,
    json_account: Json<UpdateAccount>,
    db: &State<Database>,
) -> Option<Json<Account>> {
    let id_object_result = ObjectId::from_str(id);

    let account = json_account.into_inner();

    if account.id.is_some() {
        return None;
    }

    match id_object_result {
        Err(_) => None,
        Ok(id_object) => {
            let filter = doc! { "_id": id_object };

            {
                let modifications = to_bson(&account).unwrap();
                db.accounts
                    .update_one(filter, doc! {"$set": modifications }, None)
                    .await
                    .unwrap();
            }

            read_account(id, db).await
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
        // let uri = "mongodb://localhost:27017".to_string();
        let client = Client::with_options(
            ClientOptions::builder()
                .retry_writes(false)
                .retry_reads(false)
                .build(),
        )
        .unwrap();
        // let client = Client::with_uri_str(uri).await.unwrap();
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
