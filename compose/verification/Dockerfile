FROM python:3.15-alpine
RUN apk add --no-cache openssl postgresql18-client
WORKDIR /verification
COPY compose/verification/*.py ./
ENTRYPOINT ["python", "-u"]
