#[macro_use] extern crate rocket;

use mongodb::{bson::{doc, oid::ObjectId}, Client, Collection};
use rocket::{serde::{json::Json, Deserialize, Serialize}, State};

#[get("/account/<id>")]
async fn get_account_by_id(id: &str, db: &State<Database>) -> Option<Json<Account>> {
    let filter = doc! { "_id": id };
    let maybe_account = db.accounts.find_one(filter, None).await.unwrap();

    maybe_account.map(Json)
}

#[derive(Debug, Serialize, Deserialize)]
struct Account {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    email: String,
    first_name: String,
    last_name: String,
}

struct Database {
    accounts: Collection<Account>
}

impl Database {
    async fn create() -> Self {
        let uri = "mongodb://localhost:27017".to_string();
        let client = Client::with_uri_str(uri).await.unwrap();
        let db = client.database("Example");

        let accounts: Collection<Account> = db.collection("Accounts");

        Self { accounts }
    }
}

#[launch]
async fn rocket() -> _ {
    let db = Database::create().await;

    rocket::build().manage(db).mount("/", routes![get_account_by_id])
}