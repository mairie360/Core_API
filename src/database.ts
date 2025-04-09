import { Sequelize } from 'sequelize';

const POSTGRES_URL = process.env.POSTGRES_URL || 'postgres://core:core@localhost:5432/core';

const database = new Sequelize(POSTGRES_URL, {
    dialect: 'postgres',
    logging: true,
});

export default database;