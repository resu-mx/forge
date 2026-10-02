enum QualificationKind {
    Certification,
    Credential,
}

struct Qualification {
    title: String,
    kind: QualificationKind,
    description: String,
}

struct Certification {
    title: String,
    issuer: OrganizationId, // Q: Also add 'certifier'?
    validity: CertificationValidity,
}

struct CertificationValidity {
    expires: bool,
    validity: u16,
    issued_on: Date,
    expires_on: Date,
}

struct Credential {
    title: String,
    issuer: OrganizationId, // Q: Also add 'certifier'?
    validity: CredentialValidity,
}
