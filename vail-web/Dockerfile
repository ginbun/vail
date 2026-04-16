FROM node:24-alpine AS builder

WORKDIR /app

RUN apk add --no-cache build-base zlib-dev autoconf automake libtool nasm pkgconf python3

COPY package.json package-lock.json ./
RUN npm ci --legacy-peer-deps

COPY . .
RUN npm run build

FROM nginx:1.28-alpine

COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /app/dist /usr/share/nginx/html

EXPOSE 80
