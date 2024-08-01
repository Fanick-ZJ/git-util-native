#[napi]
pub struct LogStruct {
    pub hash: String,
    pub date: String,
    pub message: String,
    pub refs: String,
    pub body: String,
    pub author_name: String,
    pub author_email: String
}