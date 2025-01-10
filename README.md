# Photo Gallery

This is a photo gallery project.

## Getting Started

### Prerequisites

- Rust and Cargo installed
- Node.js and Yarn installed

### Setup

1. Clone the repository:

   ```sh
   git clone https://github.com/yourusername/photo-gallery.git
   cd photo-gallery
   ```

2. Set up the `.env` file:

   ```sh
   cp .env.example .env
   ```

3. Configure the environment variables in the `.env` file, including the path to your SQLite database, your admin username and password, and your authetication secret.

### Backend

1. Navigate to the backend directory:

   ```sh
   cd backend
   ```

2. Run the backend server:
   ```sh
   cargo run
   ```

### Frontend

1. Navigate to the frontend directory:

   ```sh
   cd frontend
   ```

2. Install dependencies:

   ```sh
   yarn install
   ```

3. Run the frontend server:

   ```sh
   yarn dev
   ```

### Database

1. Set the `DATABASE_URL` environment variable:

    ```sh
    export DATABASE_URL="sqlite://photogallery.db"
    ```

2. Create and migrate the SQLite database using `sqlx` CLI:
    ```sh
    sqlx database create
    sqlx migrate run
    ```

## License

This project is licensed under the MIT License.
