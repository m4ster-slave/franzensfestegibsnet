use crate::models::auth::{
    create_session, destroy_session, verify_password, CurrentUser, LoginForm, RegisterForm, User,
};
use crate::models::security::{
    login_is_rate_limited, record_auth_attempt, register_is_rate_limited, ClientIp,
};
use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use sqlx::PgPool;

#[get("/register")]
pub async fn register(current_user: Option<CurrentUser>) -> Result<Template, Redirect> {
    if current_user.is_some() {
        return Err(Redirect::to("/forum"));
    }

    Ok(Template::render("register", context! {}))
}

#[post("/register", data = "<form>")]
pub async fn register_post(
    db: &State<PgPool>,
    cookies: &CookieJar<'_>,
    client_ip: ClientIp,
    form: Form<RegisterForm>,
) -> Result<Redirect, Template> {
    let form = form.into_inner();
    let ip = client_ip.0;

    if register_is_rate_limited(db.inner(), &ip)
        .await
        .unwrap_or(true)
    {
        return Err(Template::render(
            "register",
            context! { error: "Could not create account. Please try again later." },
        ));
    }

    if form.username.trim().len() < 3 {
        let _ = record_auth_attempt(db.inner(), "register", &ip, None, false).await;
        return Err(Template::render(
            "register",
            context! { error: "Username must be at least 3 characters" },
        ));
    }

    let user = match User::create(
        db.inner(),
        &form.username,
        &form.email,
        &form.password,
        "user",
    )
    .await
    {
        Ok(user) => {
            let _ = record_auth_attempt(db.inner(), "register", &ip, None, true).await;
            user
        }
        Err(_) => {
            let _ = record_auth_attempt(db.inner(), "register", &ip, None, false).await;
            return Err(Template::render(
                "register",
                context! { error: "Could not create account. The username or email may already be taken." },
            ));
        }
    };

    match create_session(db.inner(), cookies, user.id).await {
        Ok(_) => Ok(Redirect::to("/forum")),
        Err(_) => Err(Template::render(
            "login",
            context! { error: "Account created, but login failed. Please sign in." },
        )),
    }
}

#[get("/login")]
pub async fn login(current_user: Option<CurrentUser>) -> Result<Template, Redirect> {
    if current_user.is_some() {
        return Err(Redirect::to("/forum"));
    }

    Ok(Template::render("login", context! {}))
}

#[get("/profile")]
pub async fn profile(current_user: Option<CurrentUser>) -> Result<Template, Redirect> {
    let Some(current_user) = current_user else {
        return Err(Redirect::to("/login"));
    };

    Ok(Template::render(
        "profile",
        context! { current_user: current_user.0 },
    ))
}

#[post("/login", data = "<form>")]
pub async fn login_post(
    db: &State<PgPool>,
    cookies: &CookieJar<'_>,
    client_ip: ClientIp,
    form: Form<LoginForm>,
) -> Result<Redirect, Template> {
    let form = form.into_inner();
    let ip = client_ip.0;

    if login_is_rate_limited(db.inner(), &ip, &form.username_or_email)
        .await
        .unwrap_or(true)
    {
        return Err(Template::render(
            "login",
            context! { error: "Invalid username, email, or password" },
        ));
    }

    let user = match User::find_for_login(db.inner(), &form.username_or_email).await {
        Ok(Some(user))
            if !user.disabled && verify_password(&form.password, &user.password_hash) =>
        {
            let _ = record_auth_attempt(
                db.inner(),
                "login",
                &ip,
                Some(&form.username_or_email),
                true,
            )
            .await;
            user
        }
        _ => {
            let _ = record_auth_attempt(
                db.inner(),
                "login",
                &ip,
                Some(&form.username_or_email),
                false,
            )
            .await;
            return Err(Template::render(
                "login",
                context! { error: "Invalid username, email, or password" },
            ));
        }
    };

    match create_session(db.inner(), cookies, user.id).await {
        Ok(_) => Ok(Redirect::to("/forum")),
        Err(_) => Err(Template::render(
            "login",
            context! { error: "Could not start session" },
        )),
    }
}

#[post("/logout")]
pub async fn logout(db: &State<PgPool>, cookies: &CookieJar<'_>) -> Redirect {
    let _ = destroy_session(db.inner(), cookies).await;
    Redirect::to(uri!(login))
}
