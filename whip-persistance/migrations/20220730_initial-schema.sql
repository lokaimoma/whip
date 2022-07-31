CREATE TABLE Download_Task (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_name TEXT NOT NULL,
    file_size INTEGER DEFAULT 0,
    file_url TEXT NOT NULL,
    supports_resume INTEGER DEFAULT 0,
    temp_files_path TEXT NOT NULL,
    final_file_path TEXT NOT NULL,
    thread_count INTEGER NOT NULL,
    percentage_completed REAL DEFAULT 0,
    date_created TEXT NOT NULL
);
