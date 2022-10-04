-- Add migration script here

INSERT INTO
    users (
        user_id,
        username,
        password_hash
    )
VALUES (
        'ec36e61b-ac08-466d-a6e4-b13b14fafc55',
        'admin',
        '$argon2id$v=19$m=1500,t=2,p=1$8uCZPQ9prjmvoD4FGJjbjQ$7vJoq47c7uU8XFvA6Zzch+rREhYzLL0B7wZ81No8b9w'
    )
