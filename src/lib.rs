#![warn(missing_docs)]
#![forbid(unsafe_code)]

//! The purpose of this crate is to download a web page, then download
//! its linked image, Javascript, and CSS resources and embed them in
//! the HTML.
//!
//! Both async and blocking APIs are provided, making use of `reqwest`'s
//! support for both. The blocking APIs are enabled with the `blocking`
// Copyright 2021 David Young
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! feature.
//!
//! ## Examples
//!
//! ### Async
//!
//! ```no_run
//! use web_archive::archive;
//!
//! # async fn archive_async() {
//! // Fetch page and all its resources
//! let archive = archive("http://example.com").await.unwrap();
//!
//! // Embed the resources into the page
//! let page = archive.embed_resources();
//! println!("{}", page);
//! # }
//!
//! ```
//!
//! ### Blocking
//!
//! ```no_run
//! use web_archive::blocking;
//!
//! // Fetch page and all its resources
//! let archive = blocking::archive("http://example.com").unwrap();
//!
//! // Embed the resources into the page
//! let page = archive.embed_resources();
//! println!("{}", page);
//!
//! ```
//!

pub use error::Error;
pub use page_archive::PageArchive;
use parsing::parse_resource_urls;
pub use parsing::{ImageResource, Resource, ResourceMap, ResourceUrl};
use reqwest::StatusCode;
use std::convert::TryInto;
use std::fmt::Display;
use url::Url;

pub mod error;
pub mod page_archive;
pub mod parsing;

#[cfg(feature = "blocking")]
pub mod blocking;

/// The async archive function.
///
/// Takes in a URL and attempts to download the page and its resources.
/// Network errors get wrapped in [`Error`] and returned as the `Err`
/// case.
pub async fn archive<U>(url: U) -> Result<PageArchive, Error>
where
    U: TryInto<Url>,
    <U as TryInto<Url>>::Error: Display,
{
    let url: Url = url
        .try_into()
        .map_err(|e| Error::ParseError(format!("{}", e)))?;

    // Initialise client
    let client = reqwest::Client::new();

    // Fetch the page contents
    let content = client.get(url.clone()).send().await?.text().await?;

    // Determine the resources that the page needs
    let resource_urls = parse_resource_urls(&url, &content);

    // Download them
    let mut resource_map = ResourceMap::new();
    for resource_url in resource_urls {
        use ResourceUrl::*;

        let response = client.get(resource_url.url().clone()).send().await?;
        if response.status() != StatusCode::OK {
            // Skip any errors
            continue;
        }
        match resource_url {
            Image(u) => {
                resource_map.insert(
                    u,
                    Resource::Image(ImageResource {
                        data: response.bytes().await?,
                        mimetype: String::new(),
                    }),
                );
            }
            Css(u) => {
                resource_map.insert(u, Resource::Css(response.text().await?));
            }
            Javascript(u) => {
                resource_map
                    .insert(u, Resource::Javascript(response.text().await?));
            }
        }
    }

    Ok(PageArchive {
        url,
        content,
        resource_map,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::block_on;

    #[test]
    fn parse_invalid_url_async() {
        let u = "this~is~not~a~url";

        let res = block_on(archive(u));
        assert!(res.is_err());

        if let Err(Error::ParseError(_err)) = res {
            // Okay, it's a parse error
        } else {
            panic!("Expected parse error");
        }
    }
}
