//! Simple secret masking for titles derived from chat history (SPEC §11.3,
//! §17). Keeps the first 4 characters of anything that looks like a secret
//! and masks the rest. The full scanner (`kipple ai secrets`) is v1.

/// What replaces the hidden part of a secret.
const MASK: &str = "****";

/// Characters that can appear inside a token we inspect.
fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+' | '/' | '.')
}

/// Known prefixes of provider tokens, with the minimum length of the rest.
const PREFIXES: &[(&str, usize)] = &[
    ("AKIA", 16),        // AWS access key id
    ("ASIA", 16),        // AWS temporary access key id
    ("ghp_", 20),        // GitHub personal access token
    ("gho_", 20),        // GitHub OAuth token
    ("ghu_", 20),        // GitHub user-to-server token
    ("ghs_", 20),        // GitHub server-to-server token
    ("ghr_", 20),        // GitHub refresh token
    ("github_pat_", 20), // GitHub fine-grained token
    ("sk-ant-", 16),     // Anthropic API key
    ("sk-proj-", 16),    // OpenAI project key
    ("sk-", 20),         // OpenAI and other `sk-` keys
    ("xoxb-", 10),       // Slack bot token
    ("xoxp-", 10),       // Slack user token
    ("xoxa-", 10),       // Slack app token
    ("xoxr-", 10),       // Slack refresh token
    ("xoxs-", 10),       // Slack session token
    ("xapp-", 10),       // Slack app-level token
];

fn looks_like_jwt(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    parts.len() == 3
        && token.starts_with("eyJ")
        && parts.iter().all(|p| {
            p.len() >= 4
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
}

/// Shannon entropy in bits per character.
fn entropy(s: &str) -> f64 {
    let mut counts = [0u32; 128];
    let mut n = 0u32;
    for b in s.bytes().filter(|b| b.is_ascii()) {
        counts[b as usize] += 1;
        n += 1;
    }
    if n == 0 {
        return 0.0;
    }
    counts
        .iter()
        .filter(|c| **c > 0)
        .map(|c| {
            let p = f64::from(*c) / f64::from(n);
            -p * p.log2()
        })
        .sum()
}

/// Long random-looking strings: ≥ 32 characters, letters and digits mixed,
/// high entropy. Paths and plain words don't qualify.
fn looks_random(token: &str) -> bool {
    let core = token.trim_matches(|c| c == '.' || c == '-');
    if core.len() < 32 || core.contains('/') && core.matches('/').count() > 2 {
        return false;
    }
    let digits = core.chars().filter(char::is_ascii_digit).count();
    let letters = core.chars().filter(char::is_ascii_alphabetic).count();
    digits >= 4 && letters >= 8 && entropy(core) >= 3.5
}

fn is_secret(token: &str) -> bool {
    let known = PREFIXES.iter().any(|(prefix, min_rest)| {
        token
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.len() >= *min_rest)
    });
    known || looks_like_jwt(token) || looks_random(token)
}

fn mask_token(token: &str) -> String {
    let keep: String = token.chars().take(4).collect();
    format!("{keep}{MASK}")
}

/// Masks anything that looks like a secret in `text`.
pub fn mask(text: &str) -> String {
    // A PEM private key: hide everything from the header on.
    let text = match text.find("-----BEGIN") {
        Some(start) if text[start..].contains("PRIVATE KEY") => {
            format!("{}-----BEGIN PRIVATE KEY{MASK}", &text[..start])
        }
        _ => text.to_owned(),
    };
    let mut out = String::with_capacity(text.len());
    let mut token = String::new();
    let flush = |token: &mut String, out: &mut String| {
        if !token.is_empty() {
            if is_secret(token) {
                out.push_str(&mask_token(token));
            } else {
                out.push_str(token);
            }
            token.clear();
        }
    };
    for c in text.chars() {
        if is_token_char(c) {
            token.push(c);
        } else {
            flush(&mut token, &mut out);
            out.push(c);
        }
    }
    flush(&mut token, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_provider_tokens() {
        let cases = [
            ("key AKIAIOSFODNN7EXAMPLE here", "key AKIA**** here"),
            (
                "use ghp_aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789",
                "use ghp_****",
            ),
            (
                "github_pat_11ABCDEFG0123456789_abcdefghijklmnop",
                "gith****",
            ),
            (
                "OPENAI_API_KEY=sk-proj-abcdefghijklmnopqrstuv",
                "OPENAI_API_KEY=sk-p****",
            ),
            ("sk-ant-api03-abcdefghijklmnopqrstuvwxyz", "sk-a****"),
            ("token xoxb-1234567890-abcdefghij", "token xoxb****"),
        ];
        for (input, want) in cases {
            assert_eq!(mask(input), want, "{input}");
        }
    }

    #[test]
    fn masks_jwts_and_private_keys() {
        let jwt =
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.dBjftJeZ4CVPmB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(mask(&format!("bearer {jwt}")), "bearer eyJh****");
        assert_eq!(
            mask("here: -----BEGIN RSA PRIVATE KEY----- MIIEow"),
            "here: -----BEGIN PRIVATE KEY****"
        );
    }

    #[test]
    fn masks_long_random_strings() {
        assert_eq!(
            mask("secret is 9f8a7b6c5d4e3f2a1b0c9d8e7f6a5b4c3d2e1f0a"),
            "secret is 9f8a****"
        );
    }

    #[test]
    fn leaves_normal_text_alone() {
        for s in [
            "Fix the auth flow in the login page",
            "refactor src/components/authentication/LoginForm.tsx",
            "run cargo test --workspace --all-targets",
            "what does sk-learn do",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "/Users/someone/projects/my-very-long-project-name/src/main.rs",
        ] {
            assert_eq!(mask(s), s);
        }
    }
}
