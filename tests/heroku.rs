use anyhow::anyhow;
use libcnb::compress_and_put;
use reqwest::{IntoUrl, StatusCode};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct HerokuSources {
    pub source_blob: SourceUrls,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SourceUrls {
    pub get_url: String,
    pub put_url: String,
}

pub fn create_sources() -> Result<HerokuSources, anyhow::Error> {
    let client = reqwest::blocking::Client::new();
    let auth_token = std::env::var("HEROKU_AUTH_TOKEN")?;
    let app_name = "sf-package-test";
    let response = client
        .post(format!("https://api.heroku.com/apps/{}/sources", app_name).as_str())
        .header("Accept", "application/vnd.heroku+json; version=3")
        .header("Authorization", format!("Bearer {}", auth_token))
        .header("Content-Type", "application/json")
        .body(
            r#"
    {
        "source_blob": {
            "get_url":"https://s3-external-1.amazonaws.com/herokusources/...",
            "put_url":"https://s3-external-1.amazonaws.com/herokusources/..."
        }
    }"#,
        )
        .send()?;
    match response.status() {
        StatusCode::OK | StatusCode::CREATED => {
            let str = response.text()?;
            let sources: HerokuSources = serde_json::from_str(str.as_str())?;
            Ok(sources)
        }
        _ => Err(anyhow!(
            "Unexpected status {}.  {}.",
            response.status(),
            response.text()?
        )),
    }
}

pub fn upload_sources(app_dir: &PathBuf, url: impl IntoUrl) -> Result<(), anyhow::Error> {
    compress_and_put(app_dir, url)?;
    Ok(())
}

/*
Response:
{
  "app": {
    "id": "01234567-89ab-cdef-0123-456789abcdef"
  },
  "buildpacks": [
    {
      "url": "https://github.com/heroku/heroku-buildpack-ruby",
      "name": "heroku/ruby"
    }
  ],
  "created_at": "2012-01-01T12:00:00Z",
  "id": "01234567-89ab-cdef-0123-456789abcdef",
  "output_stream_url": "https://build-output.heroku.com/streams/01234567-89ab-cdef-0123-456789abcdef",
  "source_blob": {
    "checksum": "SHA256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "url": "https://example.com/source.tgz?token=xyz",
    "version": "v1.3.0"
  },
  "release": {
    "id": "01234567-89ab-cdef-0123-456789abcdef"
  },
  "slug": {
    "id": "01234567-89ab-cdef-0123-456789abcdef"
  },
  "stack": "heroku-16",
  "status": "succeeded",
  "updated_at": "2012-01-01T12:00:00Z",
  "user": {
    "id": "01234567-89ab-cdef-0123-456789abcdef",
    "email": "username@example.com"
  }
}

"{
  "app": {
    "id": "d615719c-9c34-442d-8ef7-d91b06734688"
  },
  "buildpacks": [
    {
      "url": "https://buildpack-registry.s3.amazonaws.com/buildpacks/heroku/sf-package.tgz"
    }
  ],
  "created_at": "2021-11-30T21:39:51Z",
  "id": "862110e7-9fae-49fc-b122-c940fbb8eca1",
  "output_stream_url": "https://build-output.heroku.com/streams/d6/d615719c-9c34-442d-8ef7-d91b06734688/logs/86/862110e7-9fae-49fc-b122-c940fbb8eca1.log?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIAIQI6BAUWXGR4S77Q%2F20211130%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20211130T213952Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host&X-Amz-Signature=db4a71ed1f771a3576daf2021dbf97df56772488947036e77e2e819c14a5e942",
  "release": null,
  "slug": null,
  "source_blob": {
    "checksum": null,
    "url": "https://s3-external-1.amazonaws.com/heroku-sources-production/4f065/4f065768-f333-481e-905d-eb0a6d4ad34c?AWSAccessKeyId=AKIAJ6LKZGKGPARPZE4A&Signature=HxyQc6oiTlWGITEzBx1VhqCsiuE%3D&Expires=1638311500",
    "version": "v1.0.0",
    "version_description": null
  },
  "stack": "heroku-20",
  "status": "pending",
  "updated_at": "2021-11-30T21:39:51Z",
  "user": {
    "email": "mhoefer@salesforce.com",
    "id": "ec7ef779-00e2-4002-880c-7ba1e30e6852"
  }
}"
 */
#[derive(Serialize, Deserialize, Debug)]
pub struct HerokuBuild {
    created_at: String,
    id: String,
    output_stream_url: String,
    source_blob: SourceBlob,
    status: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SourceBlob {
    url: String,
    version: String,
}

pub fn build_source(source_url: &str, source_version: &str) -> Result<HerokuBuild, anyhow::Error> {
    let client = reqwest::blocking::Client::new();
    let auth_token = std::env::var("HEROKU_AUTH_TOKEN")?;
    let app_name = "sf-package-test";
    let body_json = format!(
        r#"
    {{
      "buildpacks": [
        {{
          "url": "https://github.com/forcedotcom/sf-package-buildpack-rs.git"
        }}
      ],
      "source_blob": {{
        "url": "{}",
        "version": "{}"
      }}
    }}"#,
        source_url, source_version
    );
    let response = client
        .post(format!("https://api.heroku.com/apps/{}/builds", app_name).as_str())
        .header("Accept", "application/vnd.heroku+json; version=3")
        .header("Authorization", format!("Bearer {}", auth_token))
        .header("Content-Type", "application/json")
        .body(body_json)
        .send()?;

    match response.status() {
        StatusCode::OK | StatusCode::CREATED => {
            let str = response.text()?;
            let build: HerokuBuild = serde_json::from_str(str.as_str())?;
            Ok(build)
        }
        _ => Err(anyhow!(
            "Unexpected status {}.  {}.",
            response.status(),
            response.text()?
        )),
    }
}

pub fn build() -> Result<HerokuBuild, anyhow::Error> {
    let sources = create_sources()?;
    let put_url = sources.source_blob.put_url;
    let get_url = sources.source_blob.get_url;
    let version = "v1.0.0";

    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let app_dir = root_dir.join("tests/fixtures/sf-package");
    upload_sources(&app_dir, &put_url)?;

    build_source(get_url.as_str(), version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn it_builds() {
        let hb = build().unwrap();
        println!("{:?}", hb)
    }

    #[test]
    fn it_reads_build_response() {
        let json = r#"
            {
              "app": {
                "id": "d615719c-9c34-442d-8ef7-d91b06734688"
              },
              "buildpacks": [
                    {
                      "url": "https://buildpack-registry.s3.amazonaws.com/buildpacks/heroku/sf-package.tgz"
                    }
                ],
                "created_at": "2021-11-30T21:39:51Z",
                "id": "862110e7-9fae-49fc-b122-c940fbb8eca1",
                "output_stream_url": "https://build-output.heroku.com/streams/d6/d615719c-9c34-442d-8ef7-d91b06734688/logs/86/862110e7-9fae-49fc-b122-c940fbb8eca1.log?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIAIQI6BAUWXGR4S77Q%2F20211130%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20211130T213952Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host&X-Amz-Signature=db4a71ed1f771a3576daf2021dbf97df56772488947036e77e2e819c14a5e942",
                "release": null,
                "slug": null,
                "source_blob": {
                    "checksum": null,
                    "url": "https://s3-external-1.amazonaws.com/heroku-sources-production/4f065/4f065768-f333-481e-905d-eb0a6d4ad34c?AWSAccessKeyId=AKIAJ6LKZGKGPARPZE4A&Signature=HxyQc6oiTlWGITEzBx1VhqCsiuE%3D&Expires=1638311500",
                    "version": "v1.0.0",
                    "version_description": null
                },
                "stack": "heroku-20",
                "status": "pending",
                "updated_at": "2021-11-30T21:39:51Z",
                "user": {
                "email": "mhoefer@salesforce.com",
                "id": "ec7ef779-00e2-4002-880c-7ba1e30e6852"
            }
        }"#;

        let result: serde_json::Result<HerokuBuild> = serde_json::from_str(json);
        match result {
            Ok(build) => {
                println!("{:?}", build)
            }
            Err(e) => panic!("{:?}", e),
        }
    }
}
