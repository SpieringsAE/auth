#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{async_trait, http::StatusCode, response::{IntoResponse, Redirect}, routing::{get, post}, Form, Router};
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use go_web_interface::app::*;
    use go_web_interface::fileserv::file_and_error_handler;
    use axum_login::{
        login_required, tower_sessions::{MemoryStore, SessionManagerLayer}, AuthManagerLayerBuilder, AuthSession, AuthUser, AuthnBackend, UserId
    };
    use sha_crypt::{Sha512Params, sha512_simple, sha512_check};
    use leptos::get_configuration;
    use serde::Deserialize;

    #[derive(Clone,Deserialize)]
    struct Credentials{
        sn: String,
        client_key: String
    }

    #[derive(Clone)]
    struct User {
        id: i64,
        pw_hash: String,
    }

    impl std::fmt::Debug for User {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("User")
            .field("id", &self.id)
            .field("pw_hash", &"[redacted]")
            .finish()
        }
    }

    impl AuthUser for User {
        type Id = i64;

        fn id(&self) -> Self::Id {
            self.id
        }

        fn session_auth_hash(&self) -> &[u8] {
            self.pw_hash.as_bytes()
        }
}

    #[derive(Clone)]
    struct Backend {
        user: User,
    }

    #[async_trait]
    impl AuthnBackend for Backend {
        type User = User;
        type Credentials = Credentials;
        type Error = std::convert::Infallible;

        async fn authenticate(&self, Credentials {mut sn, client_key}: Self::Credentials) -> Result<Option<Self::User>, Self::Error> {
            sn.push_str(&client_key);
            if sha512_check(&sn, &self.user.pw_hash).is_ok() {
                Ok(Some(self.user.clone()))
            } else {
                Ok(None)
            }
        }
        
        async fn get_user(&self, _user_id: &UserId<Self>)-> Result<Option<Self::User>, Self::Error> {
            Ok(Some(self.user.clone()))
        }
    }

    async fn login(
        mut auth_session: AuthSession<Backend>,
        Form(creds): Form<Credentials>,
    ) -> impl IntoResponse {
        let user = match auth_session.authenticate(creds.clone()).await {
            Ok(Some(user)) => user,
            Ok(None) => return StatusCode::UNAUTHORIZED.into_response(),
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };

        if auth_session.login(&user).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        Redirect::to("/home").into_response()
    }

    async fn logout(
        mut auth_session: AuthSession<Backend>
    ) -> impl IntoResponse {
        match auth_session.logout().await {
            Ok(_) => Redirect::to("/").into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }

	async fn login_page(
		auth_session: AuthSession<Backend>,
	) -> impl IntoResponse {
		if auth_session.user.is_some(){
			return Redirect::to("/home").into_response();
		}
		axum::response::Html("
			<style>
			body {
				font-family: sans-serif;
				background: white;
			}
			</style>
			<title>Log in</title>
			<div style='display:flex; flex-direction:row; justify-content:center; height: 100%; overflow:auto'>
				<div style='display:flex; flex-direction:column; justify-content:center'>
					<div style='display:flex; flex-direction:row; justify-content:center'>
						<img src='/GOcontroll_logo_nontransparent.jpg' style='max-width:50%; padding-bottom:50px'/>
					</div>
					<div style='display:flex; flex-direction:row; justify-content:center'>
						<div style='background:gray; padding:20px; display:flex; flex-direction: column; justify-content:space-evenly'>
							<div style='display:flex; flex-direction:row; justify-content:center'>
								<h1>Log in</h1>
							</div>
							<form method='post' action='/login'>
							<div style='display:flex; flex-direction:row; justify-content:space-between'>
								<label for='sn' style='padding-right:10px'>Serial number </label>
								<input type='text' id='sn' name='sn' value='test'>
							</div>
							<br/>
							<div style='display:flex; flex-direction:row; justify-content:space-between'>
								<label for='client_key'>Login token </label>
								<input type='password' id='client_key' name='client_key' value='test'>
							</div>
							<br/>
							<div style='display:flex; flex-direction:row; justify-content:center'>
								<input type='submit' value='log in'>
							</div>
							</form>
						</div>
					</div>
				</div>
			</div>
		").into_response()
		
	}

    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store);
    let sha_params = Sha512Params::new(sha_crypt::ROUNDS_DEFAULT).expect("could not create hash parameters");

    let client_key = option_env!("CLIENT_KEY").unwrap_or("Moduline");
	//when deployed use the go-sn binary to get the serial number
	#[cfg(target_arch = "aarch64")]
    let mut sn = String::from_utf8(std::process::Command::new("go-sn").arg("r")
    .output().expect("Couldn't get the controllers serial number")
    .stdout).expect("serial number wasn't valid utf-8");
	//when developing set the sn to test
	#[cfg(not(target_arch = "aarch64"))]
	let mut sn = "test".to_owned();

    sn.push_str(client_key);
    let backend = Backend{
        user: User { id: 1, pw_hash: sha512_simple(&sn, &sha_params).expect("failed to create login token")}
    };
    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

    // build our application with a route
    let app = Router::new()
        .leptos_routes(&leptos_options, routes, App)
        .fallback(file_and_error_handler)
        .with_state(leptos_options)
        .route_layer(login_required!(Backend , login_url = "/"))
        .route("/login", post(login))
		.route("/", get(login_page))
        .route("/logout", post(logout))
        .layer(auth_layer);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for a purely client-side app
    // see lib.rs for hydration function instead
}
