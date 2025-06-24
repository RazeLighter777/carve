use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use oauth2::basic::{BasicErrorResponseType, BasicTokenType};
use carve::redis_manager::CompetitionStatus;

// API Response structures
#[derive(Serialize)]
pub(crate) struct UserResponse {
    pub(crate) name: String,
    pub(crate) email: String,
    #[serde(rename = "teamId")]
    pub(crate) team_id: Option<u64>, //users may not have a team
}

#[derive(Serialize)]
pub(crate) struct TeamMember {
    pub(crate) name: String,
}

#[derive(Serialize)]
pub struct CompetitionResponse {
    pub status: String,

}

#[derive(Serialize)]
pub(crate) struct TeamResponse {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) members: Vec<TeamMember>,
}

pub(crate) type OauthClient = oauth2::Client<
    oauth2::StandardErrorResponse<BasicErrorResponseType>,
    oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, BasicTokenType>,
    oauth2::StandardTokenIntrospectionResponse<oauth2::EmptyExtraTokenFields, BasicTokenType>,
    oauth2::StandardRevocableToken,
    oauth2::StandardErrorResponse<oauth2::RevocationErrorResponseType>,
    oauth2::EndpointSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointSet,
>;


#[derive(Serialize)]
pub(crate) struct LeaderboardEntry {
    #[serde(rename = "teamId")]
    pub(crate) team_id: u64,
    #[serde(rename = "teamName")]
    pub(crate) team_name: String,
    pub(crate) score: i64,
    pub(crate) rank: u64,
}

#[derive(Serialize)]
pub(crate) struct LeaderboardResponse {
    pub(crate) teams: Vec<LeaderboardEntry>,
}

#[derive(Serialize)]
pub(crate) struct BoxInfo {
    pub(crate) name: String,
}

#[derive(Serialize)]
pub(crate) struct BoxDetailResponse {
    pub(crate) name: String,
    #[serde(rename = "ipAddress")]
    pub(crate) ip_address: String,
    pub(crate) status: String,
}

#[derive(Serialize)]
pub(crate) struct BoxCredentialsResponse {
    pub(crate) name: String,
    pub(crate) username: String,
    pub(crate) password: String,
}

#[derive(Serialize)]
pub(crate) struct CheckResponse {
    pub(crate) name: String,
    pub(crate) points: u32,
}

#[derive(Serialize)]
pub(crate) struct TeamListEntry {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) members: Vec<TeamMember>,
}

#[derive(Serialize)]
pub(crate) struct TeamsResponse {
    pub(crate) teams: Vec<TeamListEntry>,
}

// Query parameters
#[derive(Deserialize)]
pub(crate) struct UserQuery {
    pub(crate) username: String,
}

#[derive(Deserialize)]
pub(crate) struct TeamQuery {
    pub(crate) id: u64,
}

#[derive(Deserialize)]
pub(crate) struct BoxesQuery {
    #[serde(rename = "teamId")]
    pub(crate) team_id: u64,
}

#[derive(Deserialize)]
pub(crate) struct BoxQuery {
    pub(crate) name: String,
}

#[derive(Deserialize)]
pub(crate) struct ScoreQuery {
    #[serde(rename = "teamId")]
    pub(crate) team_id: Option<u64>,
    #[serde(rename = "scoringCheck")]
    pub(crate) scoring_check: Option<String>,
    #[serde(rename = "startDate")]
    pub(crate) start_date: Option<DateTime<Utc>>,
    #[serde(rename = "endDate")]
    pub(crate) end_date: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct OauthCallBackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Deserialize)]
pub(crate) struct SwitchTeamQuery {
    #[serde(rename = "code")]
    pub(crate) team_join_code: u64,
}

#[derive(Deserialize)]
pub(crate) struct BoxCommandQuery {
    #[serde(rename = "boxName")]
    pub(crate) box_name: String,
    pub(crate) command: carve::redis_manager::QemuCommands,
}