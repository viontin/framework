use std::marker::PhantomData;
use crate::db::Value;
use crate::entities::Entity;
use crate::services::Service;
use crate::http::{Request, Response, StatusCode};

fn set_status(res: &mut Response, code: StatusCode) { res.status = code; }

pub trait HandlesCrud<M: Entity + serde::Serialize + 'static>: std::fmt::Debug + Send + Sync {
    fn service(&self) -> &dyn Service<M>;
    fn resource_name(&self) -> &str;
    fn before(&self, _req: &Request, _action: &str) -> Result<(), Response> { Ok(()) }
    fn after(&self, _req: &Request, _res: &mut Response, _action: &str) {}

    fn index(&self, req: Request) -> Response {
        if let Err(e) = self.before(&req, "index") { return e; }
        let items = self.service().all();
        let mut res = match items {
            Ok(v) => Response::text(&serde_json::to_string(&v).unwrap_or_else(|_| "[]".into()))
                .header("content-type", "application/json"),
            Err(e) => { let mut r = Response::html(&e.to_string()); set_status(&mut r, StatusCode::SERVER_ERROR); r }
        };
        self.after(&req, &mut res, "index"); res
    }

    fn show(&self, req: Request) -> Response {
        if let Err(e) = self.before(&req, "show") { return e; }
        let id: i64 = req.param("id").and_then(|s| s.parse().ok()).unwrap_or(0);
        let mut res = match self.service().find(id) {
            Ok(Some(item)) => Response::text(&serde_json::to_string(&item).unwrap_or_default())
                .header("content-type", "application/json"),
            Ok(None) => { let mut r = Response::html("Not found"); set_status(&mut r, StatusCode::NOT_FOUND); r }
            Err(e) => { let mut r = Response::html(&e.to_string()); set_status(&mut r, StatusCode::SERVER_ERROR); r }
        };
        self.after(&req, &mut res, "show"); res
    }

    fn store(&self, req: Request) -> Response {
        if let Err(e) = self.before(&req, "store") { return e; }
        let owned_data = match parse_json_body::<serde_json::Value>(&req) {
            Ok(serde_json::Value::Object(ref map)) => json_to_values(map),
            Ok(_) => { let mut r = Response::html("Expected JSON object"); set_status(&mut r, StatusCode::BAD_REQUEST); return r; }
            Err(e) => { let mut r = Response::html(&e); set_status(&mut r, StatusCode::BAD_REQUEST); return r; }
        };
        let data = owned_to_ref(&owned_data);
        let mut res = match self.service().create(data) {
            Ok(id) => { let mut r = Response::text(&serde_json::json!({"id": id}).to_string())
                .header("content-type", "application/json"); set_status(&mut r, StatusCode::CREATED); r }
            Err(e) => { let mut r = Response::html(&e.to_string()); set_status(&mut r, StatusCode::SERVER_ERROR); r }
        };
        self.after(&req, &mut res, "store"); res
    }

    fn update(&self, req: Request) -> Response {
        if let Err(e) = self.before(&req, "update") { return e; }
        let id: i64 = req.param("id").and_then(|s| s.parse().ok()).unwrap_or(0);
        let owned_data = match parse_json_body::<serde_json::Value>(&req) {
            Ok(serde_json::Value::Object(ref map)) => json_to_values(map),
            Ok(_) => { let mut r = Response::html("Expected JSON object"); set_status(&mut r, StatusCode::BAD_REQUEST); return r; }
            Err(e) => { let mut r = Response::html(&e); set_status(&mut r, StatusCode::BAD_REQUEST); return r; }
        };
        let mut res = match self.service().update(id, owned_to_ref(&owned_data)) {
            Ok(affected) => Response::text(&serde_json::json!({"affected": affected}).to_string())
                .header("content-type", "application/json"),
            Err(e) => { let mut r = Response::html(&e.to_string()); set_status(&mut r, StatusCode::SERVER_ERROR); r }
        };
        self.after(&req, &mut res, "update"); res
    }

    fn destroy(&self, req: Request) -> Response {
        if let Err(e) = self.before(&req, "destroy") { return e; }
        let id: i64 = req.param("id").and_then(|s| s.parse().ok()).unwrap_or(0);
        let mut res = match self.service().delete(id) {
            Ok(_) => Response::new(StatusCode::NO_CONTENT),
            Err(e) => { let mut r = Response::html(&e.to_string()); set_status(&mut r, StatusCode::SERVER_ERROR); r }
        };
        self.after(&req, &mut res, "destroy"); res
    }
}

#[derive(Debug)]
pub struct DefaultController<M: Entity + serde::Serialize + 'static, S: Service<M>> {
    pub service: S, pub resource: &'static str, _marker: PhantomData<M>,
}

impl<M: Entity + serde::Serialize + 'static, S: Service<M>> DefaultController<M, S> {
    pub fn new(service: S, resource: &'static str) -> Self {
        DefaultController { service, resource, _marker: PhantomData }
    }
}

impl<M: Entity + serde::Serialize + 'static, S: Service<M> + 'static> HandlesCrud<M> for DefaultController<M, S> {
    fn service(&self) -> &dyn Service<M> { &self.service }
    fn resource_name(&self) -> &str { self.resource }
}

fn parse_json_body<T: serde::de::DeserializeOwned>(req: &Request) -> Result<T, String> {
    serde_json::from_slice(&req.body).map_err(|e| format!("Invalid JSON: {}", e))
}

fn json_to_values(obj: &serde_json::Map<String, serde_json::Value>) -> Vec<(String, Value)> {
    obj.iter().map(|(k, v)| {
        let val = match v {
            serde_json::Value::String(s) => Value::Text(s.clone()),
            serde_json::Value::Number(n) => n.as_i64().map(Value::Int)
                .or_else(|| n.as_f64().map(Value::Float)).unwrap_or(Value::Text(n.to_string())),
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Null => Value::Null,
            other => Value::Text(other.to_string()),
        };
        (k.clone(), val)
    }).collect()
}

fn owned_to_ref(data: &[(String, Value)]) -> Vec<(&str, Value)> {
    data.iter().map(|(k, v)| (k.as_str(), v.clone())).collect()
}
