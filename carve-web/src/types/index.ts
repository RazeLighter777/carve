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

export enum CompetitionStatus {
  Active = 'active',
  Unstarted = 'unstarted',
  Finished = 'finished',
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
  start_time: number | null;
  end_time: number | null;
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
  id: number;
  teamId: number;
  scoringCheck: string;
  timestamp: string;
  message: string;
}

export interface OAuthRedirectResponse {
  redirectUrl: string;
}

export interface ApiError {
  message: string;
  status: number;
}
