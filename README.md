# Example for using the OSLC API 

# Challenges

- login using OpenID


# Dealing with OpenID

- get service provider information on _url_root_/sp
- get the auth_endpoint from osle:oauthConfiguration/oslc:OAuthConfiguration/oslc:authorizationURI
- listen for GET requests on /openid/callback
- post a link to _auth\_endpoint_?response_type=code&clientId=_client\_id_&scope=email,profile&redirect_uri=http://localhost:8888/openid/callback
- retrieve the authorization code from the query params of the callback URL (e.g. http://localhost/openid/callback?session_state=18f42600&code=eyJhbGciOiJkaXIiLCJlbmMiOiJBMTI4Q0JDLUhTMjU2In0..yP6Yee4H_4)

we now have the auth_code, now we can exchange this in an authentication code by creating a message body 

    sso=openid;code={auth_code};redirect_uri={redirect_uri}

and POST it to {root_uri}/login. The Cloud Server will check the authorization code and return an XML which contains the authentication token.

Pass this token with every request to the OSLC API.


# Useful links:

- [Authorizing Users in a Model with OpenID](https://sparxsystems.com/enterprise_architect_user_guide/14.0/model_repository/oslc_auth_users.html)
