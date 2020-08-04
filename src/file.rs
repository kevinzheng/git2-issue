use std::io::Write;

use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use git2::{IndexEntry, Oid, Repository, Signature};
use log::{error, info, trace, warn};

use crate::errors::AppError;
use crate::utils;

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateFileInput {
    file_path: String,
    file_content: String,
    commit_message: String,
    branch: String,
}

#[post("/repositories/{repo_id}/files/createFile")]
pub async fn create_file(
    repo_id: web::Path<String>,
    input: web::Json<CreateFileInput>,
) -> Result<HttpResponse, AppError> {
    let signature = Signature::now("someone", "someone@gmail.com")?;

    println!(">>>>>>>>>>>> create file input: {:?}", &input);

    let repo = utils::open_bare_repo(&repo_id)?;
    utils::create_file_then_commit_then_push(
        &repo,
        &input.file_path,
        &input.file_content,
        &input.branch,
        &signature,
        &signature,
        &input.commit_message,
    )?;

    Ok(HttpResponse::Ok().finish())
}

#[cfg(test)]
mod tests {
    use std::fs::read;
    use std::path::Path;
    use std::str::FromStr;
    use std::thread;
    use std::time::Duration;

    use actix_web::body::{Body, MessageBody, ResponseBody};
    use actix_web::dev::ServiceResponse;
    use actix_web::{test, web, App};
    use serde::de::Expected;
    use serde_json::Value;
    use urlencoding;

    use super::*;

    /// Try to create files through actix_web handler, but all old files will be deleted for known reason
    #[actix_rt::test]
    async fn test_create_file() {
        let repo_id = utils::uuid();

        println!(">>>>>>>>>>>>>>>>> repo_id: {}", &repo_id);

        let repo = utils::repo_create(&repo_id).unwrap();

        let signature = Signature::now("Someone", "someone@gmail.com").unwrap();

        // List branches through HTTP request
        let mut app =
            test::init_service(App::new().service(web::scope("/api/v1").service(create_file)))
                .await;
        let url = format!("/api/v1/repositories/{}/files/createFile", repo_id);

        let mut inputs = vec![];
        let count = 10;
        for i in 0..count {
            let input = CreateFileInput {
                file_path: utils::random_string(10),
                file_content: utils::random_string(100),
                commit_message: utils::random_string(100),
                branch: "master".to_string(),
            };

            /// All old files will be deleted, only one file left at the end
            let req = test::TestRequest::post()
                .uri(&url)
                .set_json(&input)
                .to_request();
            let resp = test::call_service(&mut app, req).await;
            println!(">>>>>> resp status: {}", resp.status().as_str());
            assert!(resp.status().is_success());

            /// Works all right with calling create_file_then_commit_then_push directly
            // utils::create_file_then_commit_then_push(
            //     &repo,
            //     &input.file_path,
            //     &input.file_content,
            //     &input.branch,
            //     &signature,
            //     &signature,
            //     &input.commit_message,
            // )
            // .unwrap();
            inputs.push(input);
        }

        let files = utils::list_files_of_branch(&repo, "master", None).unwrap();
        assert_eq!(files.len(), count);
    }
}
