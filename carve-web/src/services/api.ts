import axios from 'axios';
import type { 
  Competition, 
  User, 
  Team, 
  LeaderboardEntry, 
  ScoreboardEntry, 
  OAuthRedirectResponse 
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
  async getCompetition(): Promise<Competition> {
    const response = await api.get<Competition>('/competition');
    return response.data;
  },

  // User info
  async getCurrentUser(): Promise<User> {
    const userInfo = cookieUtils.getUserInfo();
    if (!userInfo?.username) {
      throw new Error('No user info available');
    }
    const response = await api.get<User>(`/user?username=${userInfo.username}`);
    return response.data;
  },
  async getUserTeam(): Promise<Team> {
    const userInfo = cookieUtils.getUserInfo();
    if (!userInfo?.username) {
      throw new Error('No user info available');
    }
    const user = await this.getCurrentUser();
    const response = await api.get<Team>(`/team?id=${user.teamId}`);
    return response.data;
  },
  // Leaderboard
  async getLeaderboard(): Promise<LeaderboardEntry[]> {
    const response = await api.get('/leaderboard');
    return response.data.teams || [];
  },

  // Scoreboard
  async getScoreboard(teamId?: string, boxId?: string): Promise<ScoreboardEntry[]> {
    const params = new URLSearchParams();
    if (teamId) params.append('teamId', teamId);
    if (boxId) params.append('scoringCheck', boxId);
    
    const response = await api.get(`/score?${params.toString()}`);
    return response.data || [];
  },

  // Get available teams and boxes for filters
  async getTeams(): Promise<Team[]> {
    const response = await api.get('/teams');
    return response.data.teams || [];
  },

  async getBoxes(teamId: string): Promise<Array<{name: string}>> {
    const response = await api.get(`/boxes?teamId=${teamId}`);
    return response.data || [];
  },

  async getChecks(): Promise<Array<{name: string, points: number}>> {
    const response = await api.get('/checks');
    return response.data || [];
  }
};

export default apiService;
