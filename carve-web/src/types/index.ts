export interface User {
  name: string;
  email: string;
  teamId?: number;
  is_admin: boolean;
}

export interface TeamMember {
  name: string;
}

export interface Team {
  id: number;
  name: string;
  members: TeamMember[];
}

export interface AdminGenerateTeamCodeRequest {
  team_name: string;
}

export enum CompetitionStatus {
  Active = 'active',
  Unstarted = 'unstarted',
  Finished = 'finished',
}

export interface FlagCheckStatusResponse {
    name: string;
    passing: boolean;
}

export interface CheckStatusResponse {
    name: string;
    passing: boolean;
    failed_for: number;
    message: string[];
    success_fraction: [number, number]; // Tuple of (passing, total)
    passing_boxes: string[]; // List of boxes that passed this check
}

export interface TeamCheckStatusResponse {
    checks: CheckStatusResponse[];
    flag_checks: FlagCheckStatusResponse[];
}

export interface Check {
    name: string;
    description: string;
    interval: number;
    points: number;
}

export interface FlagCheck {
    name: string;
    description: string;
    points: number;
    attempts: number;
    box_name: string;
}

export interface CheckResponse {
    checks: Check[];
    flag_checks: FlagCheck[];
}

export interface GenerateTeamCodeResponse {
  code : string;
}

export interface TeamConsoleCodeResponse {
  code: string;
}

export interface CompetitionState {
  name: string;
  status: CompetitionStatus;
  start_time: string | null;
  end_time: string | null;
}

export interface Box {
  name: string;
  ipAddress: string;
  status: string;
}

export interface TeamJoinResponse {
  team_name: string;
}

export interface LeaderboardEntry {
  teamId: number;
  teamName: string;
  score: number;
  rank: number;
}


export interface OAuthRedirectResponse {
  redirectUrl: string;
}

export interface ApiError {
  message: string;
  status: number;
}

export interface RedeemFlagQuery {
  flag: string;
  flagCheckName: string;
}

export interface RedeemFlagResponse {
  success: boolean;
  message: string;
}

export interface LoginUserQuery {
  username: string;
  password: string;
}

export interface RegistrationQuery {
  username: string;
  password: string;
  email: string;
  team_join_code?: number;
}

// Enum equivalent to Rust's IdentitySources
export enum IdentitySources {
  LocalUserPassword = 'LocalUserPassword',
  OIDC = 'OIDC',
}

// Struct equivalent to Rust's IdentitySourcesResponse
export interface IdentitySourcesResponse {
  sources: IdentitySources[];
}

// Rust: pub(crate) struct BoxRestoreQuery { pub(crate) box_name: String }
export interface BoxRestoreQuery {
  boxName: string;
}

// Rust: pub(crate) struct BoxSnapshotQuery { pub(crate) box_name: String }
export interface BoxSnapshotQuery {
  boxName: string;
}


// Rust: pub(crate) struct ScoreAtGivenTimeQuery
export interface ScoreAtGivenTimesQuery {
  teamId: number;
  scoringCheck?: string;
  atTimes: string[]; // ISO8601 strings
}

export interface ScoresAtGivenTimeResponse {
  scores: number[];
}