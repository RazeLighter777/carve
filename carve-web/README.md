# carve-web

Carve's web interface is a single page application built in vue. It uses carve-api for backend communication.

It uses vue's router for navigation.

All pages besides /login and /logout require the user to be logged in, which is checked by the presence of a userinfo cookie.

There is a navigation bar at the top of the page (besides the login page)that allows the user to navigate to the different pages of the application.

## Pages
### /login
This is the default page if the userinfo cookie is not set. It shows a simple login page with a link that allows the user to log in with OIDC.

It reads the configuration competition.oidc_provider_name to display the client name in the login page.

It then queries /api/v1/oauth2/get_oauth2_redirect_url to get the redirect URL for the OIDC provider, and sets that as the href for the login button.

The user will be redirected to the OIDC provider's login page, then to  /api/v1/oauth2/callback which will set the userinfo cookie and redirect to / if 
the login was successful, or back to /login if it failed, setting the ?error=<error> query parameter.

### /logout
Deletes the userinfo cookie and the actix-session cookie, then redirects to /login.

### /
This is the main page of the application. 
It shows the competition name and description.

It shows the current user, and table of their team members

### /leaderboard

The leaderboard shows the current scores for each team in the competition in a table using /api/v1/leaderboard.

### /scoreboard

The scoreboard shows the current score graph. 

It has drop down boxes to filter by team and by box. It uses /api/v1/scoreboard to get the data.

### /about

Information page stating the CARVE license is AGPL-3.0, and that the source code is available on GitHub.

