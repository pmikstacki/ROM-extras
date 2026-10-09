use crate::{Smtp, bounded_stream::BoundedStream};
use lettre::transport::smtp::{
    authentication::{Credentials, Mechanism},
    client::AsyncSmtpConnection,
    commands::{Data, Mail, Rcpt},
    extension::ClientId,
};
use rom::DeliveryOutcome;
use rom_email_core::PreparedEmail;
use tokio::net::TcpStream;
impl Smtp {
    pub(crate) async fn attempt(&self, message: &PreparedEmail) -> DeliveryOutcome {
        let result = self.exchange(message).await;
        match result {
            Ok(true) => DeliveryOutcome::Accepted,
            Ok(false) => DeliveryOutcome::Unknown,
            Err(Some(error)) if error.is_transient() => DeliveryOutcome::Retryable,
            Err(Some(error)) if error.is_permanent() => DeliveryOutcome::Permanent,
            Err(_) => DeliveryOutcome::Unknown,
        }
    }
    async fn exchange(
        &self,
        message: &PreparedEmail,
    ) -> Result<bool, Option<lettre::transport::smtp::Error>> {
        let stream = TcpStream::connect(self.endpoint.address)
            .await
            .map_err(|_| None)?;
        let stream = self
            .connector
            .connect(self.endpoint.identity.clone(), stream)
            .await
            .map_err(|_| None)?;
        let stream = BoundedStream::new(stream, self.limits.response_bytes);
        let mut connection = AsyncSmtpConnection::connect_with_transport(
            Box::new(stream),
            &ClientId::Domain(self.endpoint.hello.clone()),
        )
        .await
        .map_err(Some)?;
        let credentials = Credentials::new(
            self.credentials.username.to_string(),
            self.credentials.password.to_string(),
        );
        let auth = connection
            .auth(&[Mechanism::Plain], &credentials)
            .await
            .map_err(Some)?;
        if !auth.has_code(235) {
            return Ok(false);
        }
        let from = message.from().parse().map_err(|_| None)?;
        let to = message.to().parse().map_err(|_| None)?;
        let mail = connection
            .command(Mail::new(Some(from), vec![]))
            .await
            .map_err(Some)?;
        if !mail.has_code(250) {
            return Ok(false);
        }
        let recipient = connection
            .command(Rcpt::new(to, vec![]))
            .await
            .map_err(Some)?;
        if !recipient.has_code(250) && !recipient.has_code(251) {
            return Ok(false);
        }
        let readiness = connection.command(Data).await.map_err(Some)?;
        if !readiness.has_code(354) {
            return Ok(false);
        }
        let response = connection.message(message.body()).await.map_err(Some)?;
        // Drop immediately after acknowledgment; QUIT uncertainty cannot erase acceptance.
        Ok(response.has_code(250))
    }
}
