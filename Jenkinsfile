pipeline {
    agent {
        docker {image "rustlang/rust:nightly"}
    }

    stages {
        stage("Checkout") {
            steps {
                checkout scm
            }
        }
        stage("Test") {
            steps {
                sh "cargo test"
            }
        }
        stage("Clippy") {
            steps {
                sh "cargo +nightly clippy --all"
            }
        }
        stage("Rustfmt") {
            steps {
                // The build will fail if rustfmt thinks any changes are
                // required.
                sh "cargo +nightly fmt --all -- --write-mode diff"
            }
        }
        stage("Build") {
            steps {
                sh "cargo build --release"
            }
        }
    }

    post {
        always {
            archiveArtifacts artifacts: "target/release/spotsync", fingerprint: true
        }
    }
}
