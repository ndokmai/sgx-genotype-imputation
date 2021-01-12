# SMac configuration
## 0 for SMac; 1 for SMac-lite
LITE=0
## 0 for simulation-mode; 1 for SGX
SGX=1
## 0 to disable remote attestation; 1 for remote attestation
RA=1

N_THREADS=4

# Enclave configuration
## location of private key to sign enclave
ENCLAVE_HEAP_SIZE=1G
ENCLAVE_STACK_SIZE=100K
SP_SIGNING_KEY=service-provider/keys/sp_private_signing_key.pem
