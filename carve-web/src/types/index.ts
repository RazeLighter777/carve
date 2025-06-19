export interface User {
  name: string;
  email: string;
  teamId: number;
}

export interface TeamMember {
  name: string;
}

export interface Team {
  id: number;
  name: string;
  members: TeamMember[];
}

export interface Competition {
  name: string;
  status: {
    Active? : {
      start_time : number;
      end_time? : number;
    },
    Unstarted? : {},
    Finished? : {
      end_time : number;
    }
  }

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
