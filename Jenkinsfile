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
                sh "cargo +nightly fmt --all"
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
