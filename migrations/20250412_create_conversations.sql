CREATE TABLE IF NOT EXISTS conversations (
    id UUID PRIMARY KEY,
    user_message TEXT NOT NULL,
    assistant_response TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);
