enum JobSource {
    Recruiter((Contact, String)),
    JobBoardPage((JobBoard, Url)),
}

enum JobStatus {}

struct Job {
    title: String,
    organization: Organization,
    status: JobStatus,
    location: Address,
    about: String,
    candidate: JobCandidate,
    sources: Vec<&JobSource>,
    targets: Salary,
    notes: Option<Vec<&Note>>,
}

struct JobCandidate {
    requirements: Vec<&Requirement>,
    responsibilities: Vec<&Responsibilities>,
    qualification: Vec<&Qualification>,
}
