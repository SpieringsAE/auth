use crate::error_template::{AppError, ErrorTemplate};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[server(IsWifiActive, "/api")]
pub async fn is_wifi_active() -> Result<bool, ServerFnError> {
	Ok(dbg!(std::path::Path::new("/etc/modprobe.d/brcmfmac.conf").exists()))
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
		<Stylesheet id="leptos" href="/pkg/go-web-interface.css"/>

		// sets the document title
		<Title text="GOcontroll Web Interface"/>

		// content for this welcome page
		<Router fallback=|| {
			let mut outside_errors = Errors::default();
			outside_errors.insert_with_default_key(AppError::NotFound);
			view! { <ErrorTemplate outside_errors/> }.into_view()
		}>
			<main>
				<Routes>
					<Route path="/home" view=HomePage/>
				</Routes>
			</main>
		</Router>
	}
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    // Creates a reactive value to update the button
    let (count, set_count) = create_signal(0);
    let on_click = move |_| set_count.update(|count| *count += 1);
	let wifi = create_resource(|| (), |_value| async move {
		is_wifi_active().await.unwrap_or(false)
	});
	
    view! {
		<h1>"GOcontroll Moduline"</h1>
		<button on:click=on_click>"Click Me: " {count}</button>
		<form method="post" action="/logout">
			<input type="submit" value="log out"/>
		</form>
		<p>
			"Wifi: " // wait for data to load from the server
			<Suspense fallback=move || {
				view! { "loading..." }
			}>{move || if wifi.get().unwrap_or(false) { "on" } else { "off" }}
			</Suspense>
		</p>
	}
}