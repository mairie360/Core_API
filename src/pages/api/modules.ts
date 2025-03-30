import type { NextApiRequest, NextApiResponse } from 'next'

// 🔹 Response Data Type
type ResponseData = {
  status: string
  message: string
  data: Module | Module[] | null
}

// 🔹 Custom Error Classes
class ApiError extends Error {
  statusCode: number
  constructor(message: string, statusCode: number) {
    super(message)
    this.statusCode = statusCode
  }
}

class NotFoundError extends ApiError {
  constructor(message = 'Resource not found') {
    super(message, 404)
  }
}

class ConflictError extends ApiError {
  constructor(message = 'Conflict: Resource already exists') {
    super(message, 409)
  }
}

class BadRequestError extends ApiError {
  constructor(message = 'Bad Request: Invalid input') {
    super(message, 400)
  }
}

class MethodNotAllowedError extends ApiError {
  constructor(method: string) {
    super(`Method ${method} Not Allowed`, 405)
  }
}

interface Module {
  id: number
  name: string
  description: string
  version: string
  url: string
}

let modules: Module[] = [
  {
    id: 1,
    name: 'Files',
    description: 'Files module for managing documents and files.',
    version: '0.1.0',
    url: 'http://files.development.mairie360.fr',
  },
  {
    id: 2,
    name: 'Calendars',
    description: 'Calendars module for managing events and schedules.',
    version: '0.1.0',
    url: 'http://calendars.development.mairie360.fr',
  },
  {
    id: 3,
    name: 'Emails',
    description: 'Emails module for managing email communication.',
    version: '0.1.0',
    url: 'http://emails.development.mairie360.fr',
  },
  {
    id: 4,
    name: 'Projects',
    description: 'Projects module for managing tasks and projects.',
    version: '0.1.0',
    url: 'http://projects.development.mairie360.fr',
  },
  {
    id: 5,
    name: 'Messages',
    description: 'Messages module for managing internal communication.',
    version: '0.1.0',
    url: 'http://messages.development.mairie360.fr',
  },
]

// 🔹 Get All Modules
function getModules(): Module[] {
  if (modules.length === 0) {
    throw new NotFoundError('No modules found.')
  }
  return modules
}

function addModule(module: Module): void {
  if (!module.id || !module.name || !module.description || !module.version || !module.url) {
    throw new BadRequestError('Invalid module data. All fields are required.')
  }

  const existingModule = modules.find(m => m.id === module.id)
  if (existingModule) {
    throw new ConflictError(`Module with ID ${module.id} already exists.`)
  }

  modules.push(module)
}

export default function handler(
  request: NextApiRequest,
  response: NextApiResponse<ResponseData>
) {
  try {
    const { method } = request

    switch (method) {
      case 'GET': {
        const modulesList = getModules()
        response.status(200).json({ status: 'success', message: 'Modules retrieved successfully', data: modulesList })
        break
      }
      case 'POST': {
        const newModule: Module = request.body
        addModule(newModule)
        response.status(201).json({ status: 'success', message: 'Module added successfully', data: newModule })
        break
      }
      default:
        throw new MethodNotAllowedError(method!)
    }
  } catch (error) {
    if (error instanceof ApiError) {
      response.status(error.statusCode).json({ status: 'error', message: error.message, data: null })
    } else {
      console.error(error)
      response.status(500).json({ status: 'error', message: 'Internal Server Error', data: null })
    }
  }
}