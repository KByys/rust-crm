use sha2::Sha512;

fn main() {
    verify(sign())
}
fn verify(token: String) {
    use hmac::{Hmac, Mac};
    use jwt::{AlgorithmType, Header, Token, VerifyWithKey};
    use sha2::Sha384;
    use std::collections::BTreeMap;

    let key: Hmac<Sha512> = Hmac::new_from_slice(b"some-secret").unwrap();
    // let token_str = "eyJhbGciOiJIUzM4NCJ9.eyJzdWIiOiJzb21lb25lIn0.WM_WnPUkHK6zm6Wz7zk1kmIxz990Te7nlDjQ3vzcye29szZ-Sj47rLNSTJNzpQd_";

    let token: Token<Header, BTreeMap<String, String>, _> =
        VerifyWithKey::verify_with_key(token.as_ref(), &key).unwrap();
    let header = token.header();
    let claims = token.claims();
    println!("{:?}", header);
    println!("{:?}", claims)
}

fn sign() -> String {
    use hmac::{Hmac, Mac};
    use jwt::{AlgorithmType, Header, SignWithKey, Token};
    use sha2::Sha384;
    use sha2::Sha512;
    use std::collections::BTreeMap;

    let key: Hmac<Sha512> = Hmac::new_from_slice(b"some-secret").unwrap();
    let header = Header {
        algorithm: AlgorithmType::Hs512,
        ..Default::default()
    };
    let mut claims = BTreeMap::new();
    claims.insert("sub", "karl");

    let token = Token::new(header, claims).sign_with_key(&key).unwrap();
    token.as_str().into()
}
