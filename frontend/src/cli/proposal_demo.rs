use crate::governance::ProposalComment;
use crate::governance::comments::fetch_comments_threaded;
use crate::governance::comments::CommentVersion;

let comment1 = ProposalComment {
    id: comment1_id.to_string(),
    author: user_id.to_string(),
    timestamp: Utc::now(),
    content: "This looks like a good proposal! I support increasing the repair budget."
        .to_string(),
    reply_to: None,
    tags: Vec::new(),
    reactions: HashMap::new(),
    hidden: false,
    edit_history: vec![CommentVersion {
        content: "This looks like a good proposal! I support increasing the repair budget.".to_string(),
        timestamp: Utc::now(),
    }],
};

let comment2 = ProposalComment {
    id: comment2_id.to_string(),
    author: "council_member".to_string(),
    timestamp: Utc::now(),
    content:
        "I agree, but have we considered allocating some funds for preventative maintenance?"
            .to_string(),
    reply_to: Some(comment1_id.to_string()),
    tags: Vec::new(),
    reactions: HashMap::new(),
    hidden: false,
    edit_history: vec![CommentVersion {
        content: "I agree, but have we considered allocating some funds for preventative maintenance?".to_string(),
        timestamp: Utc::now(),
    }],
};

let comment3 = ProposalComment {
    id: comment3_id.to_string(),
    author: user_id.to_string(),
    timestamp: Utc::now() + Duration::seconds(30),
    content: "That's a great point about preventative maintenance. I'll allocate 20% for that purpose.".to_string(),
    reply_to: Some(comment2_id.to_string()),
    tags: Vec::new(),
    reactions: HashMap::new(),
    hidden: false,
    edit_history: vec![CommentVersion {
        content: "That's a great point about preventative maintenance. I'll allocate 20% for that purpose.".to_string(),
        timestamp: Utc::now() + Duration::seconds(30),
    }],
};

let comment4 = ProposalComment {
    id: comment4_id.to_string(),
    author: "finance_team".to_string(),
    timestamp: Utc::now() + Duration::seconds(60),
    content: "Have we verified this budget against our quarterly allocations?".to_string(),
    reply_to: None,
    tags: Vec::new(),
    reactions: HashMap::new(),
    hidden: false,
    edit_history: vec![CommentVersion {
        content: "Have we verified this budget against our quarterly allocations?".to_string(),
        timestamp: Utc::now() + Duration::seconds(60),
    }],
};

let comment5 = ProposalComment {
    id: comment5_id.to_string(),
    author: user_id.to_string(),
    timestamp: Utc::now() + Duration::seconds(90),
    content:
        "Yes, I've confirmed with accounting that this fits within our Q3 maintenance budget."
            .to_string(),
    reply_to: Some(comment4_id.to_string()),
    tags: Vec::new(),
    reactions: HashMap::new(),
    hidden: false,
    edit_history: vec![CommentVersion {
        content: "Yes, I've confirmed with accounting that this fits within our Q3 maintenance budget.".to_string(),
        timestamp: Utc::now() + Duration::seconds(90),
    }],
}; 