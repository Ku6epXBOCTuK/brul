fn main() -> Result<(), Box<dyn std::error::Error>> {
    brul::run(
        r#"
        <markup>
        window Demo (title: "BRUL", width: 480, height: 320) {
            Column (page) {
                Text (title) { "Hello from .brul" }
            }
        }
        </markup>

        <style>
        .page { padding: 24px; background: #1e1e1e; }
        .title { font-size: 24px; color: #ffffff; }
        </style>
        "#,
        None,
    )
}
