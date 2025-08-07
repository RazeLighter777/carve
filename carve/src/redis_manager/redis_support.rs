use crate::config::{SupportTicket, SupportTicketMessage, SupportTicketState, ToastNotification, ToastSeverity};
use chrono::Utc;
use regex::Regex;
use super::*;

impl RedisManager {
    /// Sanitize text input by removing HTML tags, scripts, and other potentially dangerous content
    fn sanitize_text_input(input: &str) -> String {
        let mut sanitized = input.to_string();
        
        // Remove HTML/XML tags (including script, style, etc.)
        let html_tag_regex = Regex::new(r"<[^>]*>").unwrap();
        sanitized = html_tag_regex.replace_all(&sanitized, "").to_string();
        
        // Remove potential script content between tags that might have been missed
        let script_regex = Regex::new(r"(?i)<script[^>]*>.*?</script>").unwrap();
        sanitized = script_regex.replace_all(&sanitized, "").to_string();
        
        let style_regex = Regex::new(r"(?i)<style[^>]*>.*?</style>").unwrap();
        sanitized = style_regex.replace_all(&sanitized, "").to_string();
        
        // Remove javascript: and data: URLs
        let js_url_regex = Regex::new(r"(?i)javascript\s*:").unwrap();
        sanitized = js_url_regex.replace_all(&sanitized, "").to_string();
        
        let data_url_regex = Regex::new(r"(?i)data\s*:").unwrap();
        sanitized = data_url_regex.replace_all(&sanitized, "").to_string();
        
        // Remove common XSS patterns
        let xss_patterns = [
            r"(?i)on\w+\s*=",  // onclick, onload, etc.
            r"(?i)expression\s*\(",  // CSS expressions
            r"(?i)url\s*\(",  // CSS url() that might contain javascript
        ];
        
        for pattern in &xss_patterns {
            let regex = Regex::new(pattern).unwrap();
            sanitized = regex.replace_all(&sanitized, "").to_string();
        }
        
        // Decode HTML entities to prevent double encoding issues
        sanitized = sanitized
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&#x27;", "'")
            .replace("&#x2F;", "/")
            .replace("&#x60;", "`")
            .replace("&#x3D;", "=");
        
        // Re-apply the HTML tag removal in case entities decoded to tags
        let html_tag_regex2 = Regex::new(r"<[^>]*>").unwrap();
        sanitized = html_tag_regex2.replace_all(&sanitized, "").to_string();
        
        // Trim whitespace and limit length to prevent abuse
        sanitized = sanitized.trim().to_string();
        
        // Limit message length (adjust as needed)
        if sanitized.len() > 10000 {
            sanitized.truncate(10000);
            sanitized.push_str("... [message truncated]");
        }
        
        sanitized
    }

    /// Sanitize a support ticket message
    fn sanitize_support_ticket_message(message: &str) -> String {
        Self::sanitize_text_input(message)
    }

    /// Get a support ticket by team name and ticket ID
    pub async fn get_support_ticket(
        &self,
        competition_name: &str,
        team_name: &str,
        ticket_id: u64,
    ) -> Result<Option<SupportTicket>> {
        let key = self.team_key(competition_name, team_name, "support_tickets");
        if let Some(ticket_data) = self.redis_hget::<_, _, String>(&key, ticket_id).await? {
            let ticket = Self::deserialize_from_yaml(&ticket_data)?;
            Ok(Some(ticket))
        } else {
            Ok(None)
        }
    }

    /// Get all support tickets for a team
    pub async fn get_team_support_tickets(
        &self,
        competition_name: &str,
        team_name: &str,
    ) -> Result<Vec<(u64, SupportTicket)>> {
        let key = self.team_key(competition_name, team_name, "support_tickets");
        let mut conn = self.get_connection().await?;
        let tickets: Vec<String> = redis::cmd("HGETALL")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .context("Failed to get team support tickets")?;

        let mut result = Vec::new();
        for chunk in tickets.chunks(2) {
            if chunk.len() == 2 {
                let ticket_id: u64 = chunk[0].parse()
                    .context("Failed to parse ticket ID")?;
                let ticket: SupportTicket = Self::deserialize_from_yaml(&chunk[1])?;
                result.push((ticket_id, ticket));
            }
        }

        // Sort by date (newest first)
        result.sort_by(|a, b| b.1.date.cmp(&a.1.date));
        Ok(result)
    }

    /// Get all support tickets across all teams (for admins)
    pub async fn get_all_support_tickets(
        &self,
        competition_name: &str,
    ) -> Result<Vec<(String, u64, SupportTicket)>> {
        let pattern = format!("{}:*:support_tickets", competition_name);
        let mut conn = self.get_connection().await?;
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await
            .context("Failed to get support ticket keys")?;

        let mut all_tickets = Vec::new();
        for key in keys {
            // Extract team name from key: competition:team:support_tickets
            let parts: Vec<&str> = key.split(':').collect();
            if parts.len() >= 3 {
                let team_name = parts[parts.len() - 2];
                let tickets: Vec<String> = redis::cmd("HGETALL")
                    .arg(&key)
                    .query_async(&mut conn)
                    .await
                    .context("Failed to get tickets for team")?;

                for chunk in tickets.chunks(2) {
                    if chunk.len() == 2 {
                        let ticket_id: u64 = chunk[0].parse()
                            .context("Failed to parse ticket ID")?;
                        let ticket: SupportTicket = Self::deserialize_from_yaml(&chunk[1])?;
                        all_tickets.push((team_name.to_string(), ticket_id, ticket));
                    }
                }
            }
        }

        // Sort by date (newest first)
        all_tickets.sort_by(|a, b| b.2.date.cmp(&a.2.date));
        Ok(all_tickets)
    }

    /// Create a new support ticket
    pub async fn create_support_ticket(
        &self,
        competition_name: &str,
        team_name: &str,
        initial_message: &str,
        subject: &str, // Subject of the support ticket
    ) -> Result<u64> {
        let key = self.team_key(competition_name, team_name, "support_tickets");
        let counter_key = self.team_key(competition_name, team_name, "support_ticket_counter");
        
        let mut conn = self.get_connection().await?;
        let ticket_id: u64 = redis::cmd("INCR")
            .arg(&counter_key)
            .query_async(&mut conn)
            .await
            .context("Failed to generate ticket ID")?;

        // Sanitize the initial message
        let sanitized_message = Self::sanitize_support_ticket_message(initial_message);

        let ticket = SupportTicket {
            team_name: team_name.to_string(),
            date: Utc::now(),
            state: SupportTicketState::Open,
            subject: subject.to_string(),
            messages: vec![SupportTicketMessage {
                sender: "team".to_string(),
                message: sanitized_message,
                timestamp: Utc::now(),
            }],
        };

        let ticket_data = Self::serialize_to_yaml(&ticket)?;
        self.redis_hset(&key, ticket_id, ticket_data).await?;

        // Send toast notification to all admins
        self.publish_toast(&ToastNotification {
            title: "New Support Ticket".to_string(),
            message: format!("Team '{}' created a new support ticket (#{}).", team_name, ticket_id),
            severity: ToastSeverity::Info,
            user: None,
            team: None, // Global notification for admins
        }).await?;

        Ok(ticket_id)
    }

    /// Add a message to an existing support ticket
    pub async fn add_support_ticket_message(
        &self,
        competition_name: &str,
        team_name: &str,
        ticket_id: u64,
        sender: &str, // "team" or "admin"
        message: &str,
    ) -> Result<()> {
        let key = self.team_key(competition_name, team_name, "support_tickets");
        
        if let Some(mut ticket) = self.get_support_ticket(competition_name, team_name, ticket_id).await? {
            // Sanitize the message before adding it
            let sanitized_message = Self::sanitize_support_ticket_message(message);

            ticket.messages.push(SupportTicketMessage {
                sender: sender.to_string(),
                message: sanitized_message,
                timestamp: Utc::now(),
            });

            let ticket_data = Self::serialize_to_yaml(&ticket)?;
            self.redis_hset(&key, ticket_id, ticket_data).await?;

            // Send appropriate toast notification
            if sender == "admin" {
                // Admin replied to team's ticket - notify the team
                self.publish_toast(&ToastNotification {
                    title: "Support Ticket Reply".to_string(),
                    message: format!("An administrator replied to your support ticket #{}", ticket_id),
                    severity: ToastSeverity::Info,
                    user: None,
                    team: Some(team_name.to_string()),
                }).await?;
            } else {
                // Team added a message - notify all admins
                self.publish_toast(&ToastNotification {
                    title: "Support Ticket Update".to_string(),
                    message: format!("Team '{}' added a message to support ticket #{}", team_name, ticket_id),
                    severity: ToastSeverity::Info,
                    user: None,
                    team: None, // Global notification for admins
                }).await?;
            }

            Ok(())
        } else {
            Err(anyhow::anyhow!("Support ticket {} not found", ticket_id))
        }
    }

    /// Update an entire support ticket
    pub async fn update_support_ticket(
        &self,
        competition_name: &str,
        team_name: &str,
        ticket_id: u64,
        ticket: &SupportTicket,
    ) -> Result<()> {
        let key = self.team_key(competition_name, team_name, "support_tickets");
        
        // Sanitize all messages in the ticket
        let mut sanitized_ticket = ticket.clone();
        for message in &mut sanitized_ticket.messages {
            message.message = Self::sanitize_support_ticket_message(&message.message);
        }
        
        let ticket_data = Self::serialize_to_yaml(&sanitized_ticket)?;
        self.redis_hset(&key, ticket_id, ticket_data).await?;

        // Send toast notification to the team about the update
        self.publish_toast(&ToastNotification {
            title: "Support Ticket Updated".to_string(),
            message: format!("Your support ticket #{} has been updated", ticket_id),
            severity: ToastSeverity::Info,
            user: None,
            team: Some(team_name.to_string()),
        }).await?;

        Ok(())
    }

    /// Update the status of a support ticket
    pub async fn update_support_ticket_status(
        &self,
        competition_name: &str,
        team_name: &str,
        ticket_id: u64,
        status: &str,
    ) -> Result<()> {
        let key = self.team_key(competition_name, team_name, "support_tickets");
        
        if let Some(mut ticket) = self.get_support_ticket(competition_name, team_name, ticket_id).await? {
            // Convert status to enum
            let ticket_state = match status.to_lowercase().as_str() {
                "open" => SupportTicketState::Open,
                "closed" => SupportTicketState::Closed,
                _ => return Err(anyhow::anyhow!("Invalid status: must be 'open' or 'closed'")),
            };

            ticket.state = ticket_state;
            let ticket_data = Self::serialize_to_yaml(&ticket)?;
            self.redis_hset(&key, ticket_id, ticket_data).await?;

            // Send toast notification to the team
            let status_text = match ticket.state {
                SupportTicketState::Open => "reopened",
                SupportTicketState::Closed => "closed",
            };

            self.publish_toast(&ToastNotification {
                title: format!("Support Ticket {}", match ticket.state {
                    SupportTicketState::Open => "Reopened",
                    SupportTicketState::Closed => "Closed",
                }),
                message: format!("Your support ticket #{} has been {}", ticket_id, status_text),
                severity: ToastSeverity::Info,
                user: None,
                team: Some(team_name.to_string()),
            }).await?;

            Ok(())
        } else {
            Err(anyhow::anyhow!("Support ticket {} not found", ticket_id))
        }
    }

    /// Delete a support ticket
    pub async fn delete_support_ticket(
        &self,
        competition_name: &str,
        team_name: &str,
        ticket_id: u64,
    ) -> Result<bool> {
        let key = self.team_key(competition_name, team_name, "support_tickets");
        let mut conn = self.get_connection().await?;
        let deleted: u64 = redis::cmd("HDEL")
            .arg(&key)
            .arg(ticket_id)
            .query_async(&mut conn)
            .await
            .context("Failed to delete support ticket")?;

        if deleted > 0 {
            // Send toast notification to the team
            self.publish_toast(&ToastNotification {
                title: "Support Ticket Closed".to_string(),
                message: format!("Your support ticket #{} has been closed", ticket_id),
                severity: ToastSeverity::Info,
                user: None,
                team: Some(team_name.to_string()),
            }).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get count of open support tickets for a team
    pub async fn get_team_support_ticket_count(
        &self,
        competition_name: &str,
        team_name: &str,
    ) -> Result<u64> {
        let key = self.team_key(competition_name, team_name, "support_tickets");
        let mut conn = self.get_connection().await?;
        let count: u64 = redis::cmd("HLEN")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .context("Failed to get support ticket count")?;
        Ok(count)
    }

    /// Get total count of support tickets across all teams
    pub async fn get_total_support_ticket_count(
        &self,
        competition_name: &str,
    ) -> Result<u64> {
        let pattern = format!("{}:*:support_tickets", competition_name);
        let mut conn = self.get_connection().await?;
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await
            .context("Failed to get support ticket keys")?;

        let mut total_count = 0;
        for key in keys {
            let count: u64 = redis::cmd("HLEN")
                .arg(&key)
                .query_async(&mut conn)
                .await
                .context("Failed to get ticket count for key")?;
            total_count += count;
        }

        Ok(total_count)
    }
}