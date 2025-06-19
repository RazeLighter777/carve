import Cookies from 'js-cookie';

export const cookieUtils = {
  hasUserInfo(): boolean {
    console.log('Checking for userinfo cookie');
    const userinfo = Cookies.get('userinfo');
    console.log('Userinfo cookie:', userinfo);
    return !!userinfo;
  },

  clearAuth(): void {
    Cookies.remove('userinfo');
    Cookies.remove('id');
  },

  getUserInfo(): any {
    const userinfo = Cookies.get('userinfo');
    if (!userinfo) return null;
    
    try {
      return JSON.parse(userinfo);
    } catch {
      return null;
    }
  }
};

export default cookieUtils;
