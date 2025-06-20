import axios from 'axios';
import type { 
  CompetitionState, 
  User, 
  Team, 
  LeaderboardEntry, 
  ScoreboardEntry, 
  OAuthRedirectResponse, 
  Box,
  GenerateTeamCodeResponse,
  TeamJoinResponse,
  TeamConsoleCodeResponse
} from '@/types';
import { cookieUtils } from '@/utils/cookies';
const api = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || 'http://localhost:8000/api',

  withCredentials: true,
});

// Request interceptor to handle auth
api.interceptors.request.use((config) => {
  return config;
});

// Response interceptor to handle errors
api.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      // Redirect to login if unauthorized
      window.location.href = '/login';
    }
    return Promise.reject(error);
  }
);

export const apiService = {
  // Auth endpoints
  async getOAuthRedirectUrl(): Promise<string> {
    const response = await api.get<OAuthRedirectResponse>('oauth2/get_oauth2_redirect_url');
    return response.data.redirectUrl;
  },

  // Competition info
  async getCompetition(): Promise<CompetitionState> {
    const response = await api.get<CompetitionState>('competition/competition');
    return response.data;
  },

  // User info
  async getCurrentUser(): Promise<User> {
    const userInfo = cookieUtils.getUserInfo();
    if (!userInfo?.username) {
      throw new Error('No user info available');
    }
    const response = await api.get<User>(`competition/user?username=${userInfo.username}`);
    return response.data;
  },
  async getUserTeam(): Promise<Team | undefined> {
    const userInfo = cookieUtils.getUserInfo();
    if (!userInfo?.username) {
      throw new Error('No user info available');
    }
    const user = await this.getCurrentUser();
    const response = await api.get<Team>(`competition/team?id=${user.teamId}`);
    return response.data;
  },
  // Leaderboard
  async getLeaderboard(): Promise<LeaderboardEntry[]> {
    const response = await api.get('competition/leaderboard');
    return response.data.teams || [];
  },

  // Scoreboard
  async getScoreboard(teamId?: string, boxId?: string): Promise<ScoreboardEntry[]> {
    const params = new URLSearchParams();
    if (teamId) params.append('teamId', teamId);
    if (boxId) params.append('scoringCheck', boxId);

    const response = await api.get(`competition/score?${params.toString()}`);
    return response.data || [];
  },

  // Get available teams and boxes for filters
  async getTeams(): Promise<Team[]> {
    const response = await api.get('competition/teams');
    return response.data.teams || [];
  },
  //get the team console code. no parameters needed
  async getTeamConsoleCode(): Promise<string> {
    const userInfo = cookieUtils.getUserInfo();
    if (!userInfo?.username) {
      throw new Error('No user info available');
    }
    const response = await api.get<TeamConsoleCodeResponse>(`competition/team/console_code`);
    return response.data.code;
  },

  async isUserRegisteredForAnyTeam(): Promise<boolean> {
    const userInfo = cookieUtils.getUserInfo();
    if (!userInfo?.username) {
      throw new Error('No user info available');
    }
    return userInfo.team_name !== undefined;
  },

  async getBoxes(teamId: string): Promise<Array<{name: string}>> {
    const response = await api.get(`competition/boxes?teamId=${teamId}`);
    return response.data || [];
  },
  async getBox(boxId: string): Promise<Box> {
    const response = await api.get<Box>(`competition/box?name=${boxId}`);
    return response.data || {};
  },
  async switchTeam(code : string): Promise<void> {
    const userInfo = cookieUtils.getUserInfo();
    if (!userInfo?.username) {
      throw new Error('No user info available');
    }
    const result = await api.get<TeamJoinResponse>(`competition/switch_team?code=${code}`);
    // get the team_name from the response, and update the user info cookie
    if (result.data.team_name) {
      cookieUtils.setUserInfo({
        ...userInfo,
        teamId: result.data.team_name
      });
    }
  },
  async generateJoinCode(): Promise<GenerateTeamCodeResponse> {
    const userInfo = cookieUtils.getUserInfo();
    if (!userInfo?.username) {
      throw new Error('No user info available');
    }
    const response = await api.get<GenerateTeamCodeResponse>(`competition/generate_join_code`);
    return response.data;
  },

  async getChecks(): Promise<Array<{name: string, points: number}>> {
    const response = await api.get('competition/checks');
    return response.data || [];
  },

  // Admin endpoints
  async startCompetition(): Promise<void> {
    await api.get('/admin/start_competition');
  },
  async endCompetition(): Promise<void> {
    await api.get('/admin/end_competition');
  },

  async getTeam(teamId: number): Promise<Team> {
    const response = await api.get<Team>(`competition/team?id=${teamId}`);
    return response.data;
  }
};

export default apiService;
