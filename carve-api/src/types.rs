use carve::{
    config::{Check, FlagCheck},
    redis_manager::IdentitySources,
};
use chrono::{DateTime, Utc};
use oauth2::basic::{BasicErrorResponseType, BasicTokenType};
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize)]
pub(crate) struct AdminGenerateCodeQuery {
    pub(crate) team_name: String,
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
pub(crate) struct TeamCheckStatusResponse {
    pub checks: Vec<CheckStatusResponse>,
    pub flag_checks: Vec<FlagCheckStatusResponse>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TeamCheckStatusQuery {
    #[serde(rename = "teamId")]
    pub(crate) team_id: u64,
}

#[derive(Serialize)]
pub(crate) struct CheckResponse {
    pub(crate) checks: Vec<Check>,
    pub(crate) flag_checks: Vec<FlagCheck>,
}

#[derive(Serialize)]
pub(crate) struct CheckStatusResponse {
    pub(crate) name: String,
    pub(crate) passing: bool,
    pub(crate) failed_for: u64,
    pub(crate) message: Vec<String>,
    pub(crate) success_fraction: (u64, u64), // (passing, total)
    pub(crate) passing_boxes: Vec<String>,   // List of boxes that passed this check
}

#[derive(Serialize)]
pub(crate) struct FlagCheckStatusResponse {
    pub(crate) name: String,
    pub(crate) passing: bool,
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
    pub(crate) team: Option<String>, // Optional team name for box-specific queries by admins
}

#[derive(Deserialize)]
pub(crate) struct ScoresAtGivenTimesQuery {
    #[serde(rename = "teamId")]
    pub(crate) team_id: u64,
    #[serde(rename = "scoringCheck")]
    pub(crate) scoring_check: Option<String>,
    #[serde(rename = "atTimes")]
    pub(crate) at_times: Vec<DateTime<Utc>>,
}

#[derive(Serialize)]
pub(crate) struct ScoresAtGivenTimeResponse {
    pub(crate) scores: Vec<i64>,
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
pub(crate) struct BoxRestoreQuery {
    #[serde(rename = "boxName")]
    pub(crate) box_name: String,
}

#[derive(Deserialize)]
pub(crate) struct BoxSnapshotQuery {
    #[serde(rename = "boxName")]
    pub(crate) box_name: String,
}

#[derive(Deserialize)]
pub(crate) struct RedeemFlagQuery {
    pub(crate) flag: String,
    #[serde(rename = "flagCheckName")]
    pub(crate) flag_check_name: String,
}

#[derive(Serialize)]
pub(crate) struct RedeemFlagResponse {
    pub(crate) success: bool,
    pub(crate) message: String,
}

#[derive(Deserialize)]
pub(crate) struct GenerateFlagQuery {
    #[serde(rename = "flagCheckName")]
    pub(crate) flag_check_name: String,
    #[serde(rename = "teamName")]
    pub(crate) team_name: String,
}

#[derive(Serialize)]
pub(crate) struct GenerateFlagResponse {
    pub(crate) flag: String,
}

#[derive(Deserialize)]
pub(crate) struct LoginUserQuery {
    pub(crate) username: String,
    pub(crate) password: String,
}

#[derive(Deserialize)]
pub(crate) struct RegistrationQuery {
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) email: String,
    pub(crate) team_join_code: Option<u64>,
}

#[derive(Serialize)]
pub(crate) struct IdentitySourcesResponse {
    pub(crate) sources: Vec<IdentitySources>,
}
