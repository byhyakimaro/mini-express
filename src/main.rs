use mini_express::{MiniExpress, Request, Response};

fn main() {
    let mut app = MiniExpress::new();

    app.get("/", |_: Request, mut res: Response| {
        res.send("Hello from GET /");
    });

    app.get("/private", |_: Request, mut res: Response| {
        res.status(401).send("Unauthorized access");
    });

    app.listen("127.0.0.1:3000");
}
