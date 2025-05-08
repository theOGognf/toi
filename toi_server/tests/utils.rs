use reqwest::Response;

pub async fn assert_ok_response(response: Response) -> Result<Response, String> {
    if response.status().is_success() {
        Ok(response)
    } else {
        let body = response.text().await.map_err(|err| format!("{err:?}"))?;
        Err(body)
    }
}
