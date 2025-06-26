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
    message: string;
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

export interface ScoreboardEntry {
  team_id: number;
  score_event_type: string;
  timestamp: string;
  message: string;
  box_name: string;
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
