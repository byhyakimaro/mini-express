use mini_express::{MiniExpress, Request, Response, Serialize};

fn main() {
    let mut app = MiniExpress::new();

    app.get("/", |_: Request, mut res: Response| {
        res
            .header("Content-Type", "text/plain")
            .send("Hello from GET /");
    });

    app.get("/private", |_: Request, mut res: Response| {
        res
            .header("Content-Type", "text/plain")
            .status(401)
            .send("Unauthorized access");
    });
    
    app.get("/json", |_, mut res| {
        #[derive(Serialize)]
        struct Message {
            message: String,
        }

        let data = Message { message: "Hello, JSON!".to_string() };
        res.json(&data);
    });

    app.listen("127.0.0.1:3000");
}
