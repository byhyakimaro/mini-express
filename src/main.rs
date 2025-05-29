use mini_express::{MiniExpress, Request, Response, Serialize, HashMap, Arc};

fn main() {
    let mut app = MiniExpress::new();
    
    app.use_middleware(Arc::new(|req: Request, res, params, next| {
        println!("LOG: {} {}", req.method, req.path);
        next(req, res, params);
    }));

    app.get("/", |_: Request, mut res: Response, _: Option<HashMap<String, String>>|  {
        res
            .header("Content-Type", "text/plain")
            .send("Hello from GET /");
    });

    app.get("/private", |_: Request, mut res: Response, _: Option<HashMap<String, String>>|  {
        res
            .header("Content-Type", "text/plain")
            .status(401)
            .send("Unauthorized access");
    });
    
    app.get("/json", |_, mut res: Response, _: Option<HashMap<String, String>>|  {
        #[derive(Serialize)]
        struct Message {
            message: String,
        }

        let data = Message { message: "Hello, JSON!".to_string() };
        res.json(&data);
    });
    
    app.get("/user/:id", |_: Request, mut res: Response, params: Option<HashMap<String, String>>| {
        if let Some(params) = params {
            let id = params.get("id").unwrap();
            res.send(&format!("User id: {}", id));
        } else {
            res.status(400).send("Missing id param");
        }
    });

    app.listen("127.0.0.1:3000");
}
