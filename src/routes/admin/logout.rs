use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;

use crate::{
    session_state::TypedSession,
    utils::{e500, login_page},
};

pub async fn logout(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(e500)?.is_none() {
        Ok(login_page())
    } else {
        session.logout();
        FlashMessage::info("You have successfully logged out.").send();
        Ok(login_page())
    }
}
