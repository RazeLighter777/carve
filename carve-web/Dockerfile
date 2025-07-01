# Use official Node.js image for build
FROM node:24-alpine3.21 AS build

WORKDIR /app

# Copy only carve-web's package files for install
COPY ./carve-web/package.json ./carve-web/package-lock.json ./
RUN npm install

# Copy the carve-web source code into the build context
COPY carve-web ./

RUN npm run build

# Use nginx to serve the built files
FROM nginx:alpine

# Copy built assets from previous stage
COPY --from=build /app/dist /usr/share/nginx/html

COPY ./carve-web/nginx.conf /etc/nginx/conf.d/default.conf
# Copy custom nginx config (optional)
# COPY nginx.conf /etc/nginx/nginx.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]