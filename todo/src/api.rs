// use ntex_files::NamedFile;
use ntex::http;
use ntex::web::{self, error, Error, HttpResponse};
use ntex_session::Session;
use serde::Deserialize;
use tera::{Context, Tera};

use crate::db;
use crate::session::{self, FlashMessage};

pub async fn index(
    pool: web::types::State<db::PgPool>,
    tmpl: web::types::State<Tera>,
    session: Session,
) -> Result<HttpResponse, Error> {
    let pool = (&*pool).clone();
    let tasks = web::block(move || db::get_all_tasks(&pool)).await?;

    let mut context = Context::new();
    context.insert("tasks", &tasks);

    //Session is set during operations on other endpoints
    //that can redirect to index
    if let Some(flash) = session::get_flash(&session)? {
        context.insert("msg", &(flash.kind, flash.message));
        session::clear_flash(&session);
    }

    let rendered = tmpl
        .render("index.html.tera", &context)
        .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().body(rendered))
}

#[derive(Deserialize)]
pub struct CreateForm {
    description: String,
}

pub async fn create(
    params: web::types::Form<CreateForm>,
    pool: web::types::State<db::PgPool>,
    session: Session,
) -> Result<HttpResponse, Error> {
    let pool = (&*pool).clone();

    if params.description.is_empty() {
        session::set_flash(
            &session,
            FlashMessage::error("Description cannot be empty"),
        )?;
        Ok(redirect_to("/"))
    } else {
        web::block(move || db::create_task(params.into_inner().description, &pool))
            .await?;
        session::set_flash(&session, FlashMessage::success("Task successfully added"))?;
        Ok(redirect_to("/"))
    }
}

#[derive(Deserialize)]
pub struct UpdateParams {
    id: i32,
}

#[derive(Deserialize)]
pub struct UpdateForm {
    _method: String,
}

pub async fn update(
    db: web::types::State<db::PgPool>,
    params: web::types::Path<UpdateParams>,
    form: web::types::Form<UpdateForm>,
    session: Session,
) -> Result<HttpResponse, Error> {
    match form._method.as_ref() {
        "put" => toggle(db, params).await,
        "delete" => delete(db, params, session).await,
        unsupported_method => {
            let msg = format!("Unsupported HTTP method: {}", unsupported_method);
            Err(error::ErrorBadRequest(msg).into())
        }
    }
}

async fn toggle(
    pool: web::types::State<db::PgPool>,
    params: web::types::Path<UpdateParams>,
) -> Result<HttpResponse, Error> {
    let pool = (&*pool).clone();
    web::block(move || db::toggle_task(params.id, &pool)).await?;
    Ok(redirect_to("/"))
}

async fn delete(
    pool: web::types::State<db::PgPool>,
    params: web::types::Path<UpdateParams>,
    session: Session,
) -> Result<HttpResponse, Error> {
    let pool = (&*pool).clone();
    web::block(move || db::delete_task(params.id, &pool)).await?;
    session::set_flash(&session, FlashMessage::success("Task was deleted."))?;
    Ok(redirect_to("/"))
}

fn redirect_to(location: &str) -> HttpResponse {
    HttpResponse::Found()
        .header(http::header::LOCATION, location)
        .finish()
}

// pub fn bad_request<B>(res: dev::WebResponse<B>) -> Result<ErrorHandlerResponse<B>> {
//     let new_resp = NamedFile::open("static/errors/400.html")?
//         .set_status_code(res.status())
//         .into_response(res.request())?;
//     Ok(ErrorHandlerResponse::Response(
//         res.into_response(new_resp.into_body()),
//     ))
// }

// pub fn not_found<B>(res: dev::ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
//     let new_resp = NamedFile::open("static/errors/404.html")?
//         .set_status_code(res.status())
//         .into_response(res.request())?;
//     Ok(ErrorHandlerResponse::Response(
//         res.into_response(new_resp.into_body()),
//     ))
// }

// pub fn internal_server_error<B>(
//     res: dev::ServiceResponse<B>,
// ) -> Result<ErrorHandlerResponse<B>> {
//     let new_resp = NamedFile::open("static/errors/500.html")?
//         .set_status_code(res.status())
//         .into_response(res.request())?;
//     Ok(ErrorHandlerResponse::Response(
//         res.into_response(new_resp.into_body()),
//     ))
// }
