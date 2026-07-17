FROM node:20-bullseye-slim
WORKDIR /usr/src/control-plane
COPY package.json package-lock.json ./
RUN npm install --production
COPY . .
EXPOSE 8080
CMD ["node", "server.js"]
