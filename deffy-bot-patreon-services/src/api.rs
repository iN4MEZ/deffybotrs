use crate::{ApiError, PatreonError, PatreonResult};
use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};
use std::{env, sync::Arc};
use url::Url;

static BASE_URI: &str = "https://www.patreon.com";

#[derive(Debug, Default)]
pub struct PatreonApi {
    pub access_token: String,
    pub agent: Arc<reqwest::Client>,
}

impl PatreonApi {
    pub async fn current_user(&self) -> PatreonResult<User> {
        let mut url = Url::parse(BASE_URI).unwrap();
        url.set_path("/api/oauth2/api/current_user");
        let request = self.agent.get(url);
        self.call_data(request).await
    }

    pub async fn all_members(&self) -> PatreonResult<Vec<Member>> {
        let mut url = Url::parse(BASE_URI).unwrap();

        let campaign_id = env::var("CAMPAIGN_ID").expect("Expected a token in the environment").to_string();

        url.set_path(format!("/api/oauth2/v2/campaigns/{campaign_id}/members").as_str());

        url.query_pairs_mut()
            .append_pair("fields[member]", "full_name,email,patron_status");
        let request = self.agent.get(url);

        let first: PatreonResult<(Vec<Member>, Link<String>)> = self.call_data_and_link(request).await;

        let mut all_data = Vec::new();
        let (members, mut link) = match first {
            Ok(res) => res,
            Err(e) => return Err(e),
        };

        all_data.extend(members);

        while let Some(next_url) = link.next {
            if next_url.is_empty() {
                break;
            }

            //println!("Next page available at: {}", next_url);

            // ส่งคำขอใหม่ด้วย next_url
            let next_request = Url::parse(&next_url).unwrap();
            let request = self.agent.get(next_request);
            let result: PatreonResult<(Vec<Member>, Link<String>)> = self.call_data_and_link(request).await;

            match result {
                Ok((new_members, new_link)) => {
                    all_data.extend(new_members);
                    link = new_link;
                }
                Err(e) => return Err(e),
            }
        }

        PatreonResult::Ok(all_data)
    }

    pub async fn identity(&self) -> PatreonResult<User> {
        self.call_data(self.identity_request(None)).await
    }

    pub async fn identity_include_memberships(&self) -> PatreonResult<(User, Vec<Member>)> {
        self.call_data(self.identity_request(IdentityIncldue::Memberships))
            .await
    }

    pub async fn identity_include_campaign(&self) -> PatreonResult<(User, Vec<Campaign>)> {
        self.call_data_and_include(self.identity_request(IdentityIncldue::Campaign))
            .await
    }

    fn identity_request(
        &self,
        include: impl Into<Option<IdentityIncldue>>,
    ) -> reqwest::RequestBuilder {
        let mut url = Url::parse(BASE_URI).unwrap();
        url.set_path("api/oauth2/v2/identity");
        let include = include.into();
        if let Some(include) = include {
            // url.query_pairs_mut()
            //     .append_pair("include", include.as_str());
            match include {
                IdentityIncldue::Memberships => {

                    let campaign_id = env::var("CAMPAIGN_ID").expect("Expected a token in the environment").to_string();

                    url.set_path(format!("/api/oauth2/v2/campaigns/{campaign_id}/members").as_str());

                    url.query_pairs_mut()
                        .append_pair("fields[member]", "full_name");
                }
                IdentityIncldue::Campaign => {
                    url.query_pairs_mut()
                        .append_pair("include", include.as_str());
                    url.query_pairs_mut().append_pair(
                    "fields[user]",
                    "first_name,last_name,full_name,vanity,email,about,image_url,thumb_url,created,url",
                     );

                    url.query_pairs_mut()
                        .append_pair("fields[campaign]", "created_at,creation_name,discord_server_id,google_analytics_id,has_rss,has_sent_rss_notify,image_small_url,image_url,is_charged_immediately,is_monthly,is_nsfw,main_video_embed,main_video_url,one_liner,patron_count,pay_per_name,pledge_url,published_at,rss_artwork_url,rss_feed_title,show_earnings,summary,thanks_embed,thanks_msg,thanks_video_url,url,vanity");
                }
            }
        }
        self.agent.get(url)
    }

    pub async fn webhooks(&self) -> PatreonResult<String> {
        let mut url = Url::parse(BASE_URI).unwrap();
        url.set_path("api/oauth2/v2/webhooks");
        self.api_call(self.webhooks_request(None)).await
    }

    fn webhooks_request(
        &self,
        include: impl Into<Option<WebhookIncldue>>,
    ) -> reqwest::RequestBuilder {
        let mut url = Url::parse(BASE_URI).unwrap();
        url.set_path("api/oauth2/v2/webhooks");
        let include = include.into();
        if let Some(include) = include {
            url.query_pairs_mut()
                .append_pair("include", include.as_str());
        }
        self.agent.get(url)
    }

    async fn api_call(&self, request: reqwest::RequestBuilder) -> PatreonResult<String> {
        let request = request
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("User-Agent", "Patreon-rust")
            .build()?;
        tracing::debug!("REQUEST : {} : {}", request.method(), request.url());
        let response = self.agent.execute(request).await?;
        let status = response.status();
        let text = response.text().await?;
        tracing::debug!("RESPONSE : {status} : {text}");
        if status.is_success() {
            Ok(text)
        } else {
            Err(PatreonError::PatreonApi(
                status,
                serde_json::from_str::<ApiErrorResponse>(text.as_str())?.errors,
            ))
        }
    }

    async fn call_data<T: for<'de> serde::Deserialize<'de>>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> PatreonResult<T> {
        let json = self.api_call(request).await?;

        //println!("{}",json.as_str());
        DocResponse::parse(json.as_str())
    }

    async fn call_data_and_link<D: for<'de> serde::Deserialize<'de>,
    L: for<'de> serde::Deserialize<'de> + Default>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> PatreonResult<(D,L)> {
        let json = self.api_call(request).await?;

        //println!("{}",json.as_str());
        let response = serde_json::from_str::<DocResponseWithLink<D, L>>(json.as_str())?;

        Ok((response.data, response.links.unwrap_or_default()))
    }

    async fn call_data_and_include<
        D: for<'de> serde::Deserialize<'de>,
        I: for<'de> serde::Deserialize<'de> + Default,
    >(
        &self,
        request: reqwest::RequestBuilder,
    ) -> PatreonResult<(D, Vec<I>)> {
        let json = self.api_call(request).await?;
        let response = serde_json::from_str::<DocResponseInclude<D, I>>(json.as_str())?;
        Ok((response.data, response.included))
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct DocResponse<D> {
    data: D
}

impl<D> DocResponse<D>
where
    D: for<'de> serde::Deserialize<'de>,
{
    pub(crate) fn parse(response: impl AsRef<[u8]>) -> PatreonResult<D> {
        Ok(serde_json::from_slice::<DocResponse<D>>(response.as_ref())?.data)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct DocResponseWithLink<D,L> {
    data: D,
    #[serde(default)]
    links: Option<L>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct DocResponseInclude<D, I> {
    data: D,
    #[serde(default)]
    // if not default and identity?include=campaign and not has it access in scopes be "missing field `included`"
    included: Vec<I>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct  Link<L> {
    pub next: Option<L>
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiDocument<A> {
    #[serde(rename = "type")]
    pub document_type: String,
    pub id: String,
    pub attributes: A,
}

pub type User = ApiDocument<UserAttributes>;
pub type Member = ApiDocument<MemberAttributes>;
pub type Campaign = ApiDocument<CampaignAttributes>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserAttributes {
    pub first_name: String,
    pub last_name: String,
    pub full_name: String,
    pub vanity: Option<String>,
    pub email: String,
    pub about: Option<String>,
    pub facebook_id: Option<String>,
    pub image_url: String,
    pub thumb_url: String,
    pub youtube: Option<String>,
    pub twitter: Option<String>,
    pub facebook: Option<String>,
    pub created: DateTime<Utc>,
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberAttributes {
    // pub campaign_lifetime_support_cents: i64,
    // pub currently_entitled_amount_cents: i64,
    pub email: Option<String>,
    pub full_name: String,
    // pub is_follower: bool,
    // pub last_charge_date: Option<DateTime<Utc>>,
    // pub last_charge_status: Option<LastChrgeStatus>,
    // pub lifetime_support_cents: i64,
    // pub next_charge_date: Option<DateTime<Utc>>,
    // pub note: String,
    pub patron_status: Option<PatronStatus>,
    // pub pledge_cadence: Option<i64>,
    // pub pledge_relationship_start: DateTime<Utc>,
    // pub will_pay_amount_cents: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignAttributes {
    pub created_at: DateTime<Utc>,
    pub creation_name: String,
    pub discord_server_id: Option<String>,
    pub google_analytics_id: Option<String>,
    pub has_rss: bool,
    pub has_sent_rss_notify: bool,
    pub image_small_url: String,
    pub image_url: Option<String>,
    pub is_charged_immediately: bool,
    pub is_monthly: bool,
    pub is_nsfw: bool,
    pub main_video_embed: Option<String>,
    pub main_video_url: Option<String>,
    pub one_liner: Option<String>,
    pub patron_count: i64,
    pub pay_per_name: String,
    pub pledge_url: String,
    pub published_at: Option<DateTime<Utc>>,
    pub rss_artwork_url: Option<String>,
    pub rss_feed_title: Option<String>,
    pub show_earnings: Option<bool>,
    pub summary: Option<String>,
    pub thanks_embed: Option<String>,
    pub thanks_msg: Option<String>,
    pub thanks_video_url: Option<String>,
    pub url: String,
    pub vanity: String,
}

#[derive(Serialize, Deserialize)]
struct ApiErrorResponse {
    pub errors: Vec<ApiError>,
}

macro_rules! enum_str {
    ($name:ident { $($variant:ident($str:expr), )* }) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $name {
            $($variant,)*
        }

        impl $name {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $( $name::$variant => $str, )*
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $( $name::$variant => write!(f,"{}",$str), )*
                }
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: ::serde::Serializer,
            {
                // 将枚举序列化为字符串。
                serializer.serialize_str(match *self {
                    $( $name::$variant => $str, )*
                })
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where D: ::serde::Deserializer<'de>,
            {
                struct Visitor;

                impl<'de> ::serde::de::Visitor<'de> for Visitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                        write!(formatter, "a string for {}", stringify!($name))
                    }

                    fn visit_str<E>(self, value: &str) -> Result<$name, E>
                        where E: ::serde::de::Error,
                    {
                        match value {
                            $( $str => Ok($name::$variant), )*
                            _ => Err(E::invalid_value(::serde::de::Unexpected::Other(
                                &format!("unknown {} variant: {}", stringify!($name), value)
                            ), &self)),
                        }
                    }
                }

                // 从字符串反序列化枚举。
                deserializer.deserialize_str(Visitor)
            }
        }
    };
    ($name:ident { $($variant:ident,)* } ) => {
        enum_str!(
            $name {
             $($variant(stringify!($variant)),)*
            }
        );
    };
}

enum_str!(IdentityIncldue {
    Memberships("memberships"),
    Campaign("campaign"),
});

enum_str!(LastChrgeStatus {
    Paid,
    Declined,
    Deleted,
    Pending,
    Refunded,
    Fraud,
    Other,
});

enum_str!(PatronStatus {
    ActivePatron("active_patron"),
    DeclinedPatron("declined_patron"),
    FormerPatron("former_patron"),
    Null("null"),
});

pub type Pledge = ApiDocument<PledgeAttributes>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PledgeAttributes {
    pub amount_cents: i64,
    pub created_at: DateTime<Utc>,
    pub currency: String,
    pub declined_since: Option<DateTime<Utc>>,
    pub patron_pays_fees: bool,
    pub pledge_cap_cents: i64,
}

enum_str!(WebhookIncldue {
    Client("client"),
    Campaign("campaign"),
});
