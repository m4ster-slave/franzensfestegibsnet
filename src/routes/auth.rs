use crate::models::auth::{
    create_session, destroy_session, verify_password, CurrentUser, LoginForm, RegisterForm, User,
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
    form: Form<RegisterForm>,
) -> Result<Redirect, Template> {
    let form = form.into_inner();

    if form.username.trim().len() < 3 {
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
        Ok(user) => user,
        Err(_) => {
            return Err(Template::render(
                "register",
                context! { error: "Could not create account. The username or email may already be taken." },
            ))
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
    form: Form<LoginForm>,
) -> Result<Redirect, Template> {
    let form = form.into_inner();
    let user = match User::find_for_login(db.inner(), &form.username_or_email).await {
        Ok(Some(user))
            if !user.disabled && verify_password(&form.password, &user.password_hash) =>
        {
            user
        }
        _ => {
            return Err(Template::render(
                "login",
                context! { error: "Invalid username, email, or password" },
            ))
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
